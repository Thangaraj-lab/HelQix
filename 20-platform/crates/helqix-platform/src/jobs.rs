//! Bounded execution queue with expiring worker leases and fencing tokens.
use std::collections::BTreeMap;
use crate::access::ResourceScope;

pub type JobId = String;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState { Queued, Leased { worker_id: String, token: u64, lease_until_ms: u64 }, Succeeded, Failed { reason: String }, Cancelled }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job { pub id: JobId, pub scope: ResourceScope, pub operation: String, pub payload_bytes: usize, pub submitted_at_ms: u64, pub attempts: u32, pub max_attempts: u32, pub state: JobState }
impl Job {
    pub fn new(id: impl Into<String>, scope: ResourceScope, operation: impl Into<String>, payload_bytes: usize, submitted_at_ms: u64, max_attempts: u32) -> Result<Self, QueueError> {
        let id=id.into(); let operation=operation.into();
        if id.trim().is_empty() || operation.trim().is_empty() || max_attempts == 0 { return Err(QueueError::InvalidJob); }
        Ok(Self { id, scope, operation, payload_bytes, submitted_at_ms, attempts: 0, max_attempts, state: JobState::Queued })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease { pub job: Job, pub worker_id: String, pub token: u64, pub lease_until_ms: u64 }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError { InvalidJob, EmptyWorker, DuplicateJob, QueueFull { maximum: usize }, ZeroLease, LeaseTimeOverflow, NoSuchJob, NotQueued, NotLeased, LeaseExpired, StaleLease, TokenOverflow, AttemptOverflow, ClockReversed, EmptyFailureReason }
#[derive(Debug, Clone)]
pub struct JobQueue { jobs: BTreeMap<JobId, Job>, order: Vec<JobId>, maximum_queued: usize, next_token: u64, last_now_ms: Option<u64> }
impl JobQueue {
    pub fn new(maximum_queued: usize) -> Result<Self, QueueError> { if maximum_queued == 0 { return Err(QueueError::QueueFull { maximum: 0 }); } Ok(Self { jobs:BTreeMap::new(), order:Vec::new(), maximum_queued, next_token:0, last_now_ms:None }) }
    pub fn submit(&mut self, job: Job) -> Result<(), QueueError> {
        // Job fields are public for adapters, so validate again at the queue
        // boundary instead of assuming every caller used `Job::new`.
        if job.id.trim().is_empty() || job.operation.trim().is_empty() || job.max_attempts == 0 || job.attempts != 0 || job.state != JobState::Queued { return Err(QueueError::InvalidJob); }
        if self.jobs.contains_key(&job.id) { return Err(QueueError::DuplicateJob); }
        let queued=self.jobs.values().filter(|j| j.state == JobState::Queued).count();
        if queued >= self.maximum_queued { return Err(QueueError::QueueFull { maximum:self.maximum_queued }); }
        self.order.push(job.id.clone()); self.jobs.insert(job.id.clone(),job); Ok(())
    }
    pub fn lease_next(&mut self, worker_id: impl Into<String>, now_ms: u64, lease_ms: u64) -> Result<Option<Lease>, QueueError> {
        let worker_id=worker_id.into(); if worker_id.trim().is_empty() { return Err(QueueError::EmptyWorker); }
        if lease_ms == 0 { return Err(QueueError::ZeroLease); }
        self.check_time(now_ms)?;
        self.recover_expired(now_ms)?;
        let until=now_ms.checked_add(lease_ms).ok_or(QueueError::LeaseTimeOverflow)?;
        let id=self.order.iter().find(|id| self.jobs.get(*id).is_some_and(|j| j.state == JobState::Queued)).cloned();
        let Some(id)=id else { self.last_now_ms=Some(now_ms); return Ok(None); };
        let next_attempts = self.jobs.get(&id).expect("ordered job exists").attempts.checked_add(1).ok_or(QueueError::AttemptOverflow)?;
        self.next_token=self.next_token.checked_add(1).ok_or(QueueError::TokenOverflow)?;
        let token=self.next_token;
        let job=self.jobs.get_mut(&id).expect("ordered job exists"); job.attempts=next_attempts; job.state=JobState::Leased{worker_id:worker_id.clone(),token,lease_until_ms:until};
        self.last_now_ms=Some(now_ms);
        Ok(Some(Lease{job:job.clone(),worker_id,token,lease_until_ms:until}))
    }
    pub fn renew(&mut self, id: &str, worker_id: &str, token: u64, now_ms: u64, lease_ms: u64) -> Result<u64, QueueError> {
        if lease_ms==0 { return Err(QueueError::ZeroLease); } self.check_time(now_ms)?; self.recover_expired(now_ms)?;
        let until=now_ms.checked_add(lease_ms).ok_or(QueueError::LeaseTimeOverflow)?;
        let job=self.jobs.get_mut(id).ok_or(QueueError::NoSuchJob)?;
        match &mut job.state { JobState::Leased{worker_id:owner,token:active,lease_until_ms} if owner==worker_id && *active==token => { *lease_until_ms=until; self.last_now_ms=Some(now_ms); Ok(until) }, JobState::Leased{..} => Err(QueueError::StaleLease), _=>Err(QueueError::NotLeased) }
    }
    pub fn finish(&mut self, id: &str, worker_id: &str, token: u64, now_ms: u64, result: Result<(),String>, retryable: bool) -> Result<(),QueueError> {
        self.check_time(now_ms)?; self.recover_expired(now_ms)?;
        let job=self.jobs.get_mut(id).ok_or(QueueError::NoSuchJob)?;
        let valid=matches!(&job.state,JobState::Leased{worker_id:owner,token:active,..} if owner==worker_id && *active==token);
        if !valid { return match job.state { JobState::Leased{..}=>Err(QueueError::StaleLease), _=>Err(QueueError::NotLeased) }; }
        match result { Ok(())=>job.state=JobState::Succeeded, Err(reason)=>{if reason.trim().is_empty(){return Err(QueueError::EmptyFailureReason)} if retryable && job.attempts<job.max_attempts {job.state=JobState::Queued;} else {job.state=JobState::Failed{reason};}} }
        self.last_now_ms=Some(now_ms); Ok(())
    }
    pub fn cancel(&mut self,id:&str)->Result<(),QueueError>{let job=self.jobs.get_mut(id).ok_or(QueueError::NoSuchJob)?;match job.state{JobState::Queued|JobState::Leased{..}=>{job.state=JobState::Cancelled;Ok(())},_=>Err(QueueError::NotQueued)}}
    pub fn get(&self,id:&str)->Option<&Job>{self.jobs.get(id)}
    pub fn iter(&self)->impl Iterator<Item=&Job>{self.order.iter().filter_map(|id|self.jobs.get(id))}
    fn check_time(&self,now_ms:u64)->Result<(),QueueError>{if self.last_now_ms.is_some_and(|last|now_ms<last){Err(QueueError::ClockReversed)}else{Ok(())}}
    fn recover_expired(&mut self,now_ms:u64)->Result<(),QueueError>{for job in self.jobs.values_mut(){if let JobState::Leased{lease_until_ms,..}=job.state {if lease_until_ms<=now_ms {job.state=if job.attempts<job.max_attempts {JobState::Queued}else{JobState::Failed{reason:"worker lease expired after maximum attempts".into()}};}}}self.last_now_ms=Some(now_ms);Ok(())}
}
impl std::fmt::Display for QueueError { fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { write!(f,"{self:?}") } }
impl std::error::Error for QueueError {}

#[cfg(test)]
mod tests {
    use super::*;
    fn job(id:&str)->Job { Job::new(id,ResourceScope::project("w","p"),"analyze",12,100,2).unwrap() }
    #[test] fn expired_lease_is_reissued_with_a_new_fencing_token(){let mut q=JobQueue::new(2).unwrap();q.submit(job("a")).unwrap();let first=q.lease_next("worker-1",100,10).unwrap().unwrap();let second=q.lease_next("worker-2",110,10).unwrap().unwrap();assert_ne!(first.token,second.token);assert_eq!(second.job.attempts,2);assert_eq!(q.finish("a","worker-1",first.token,110,Ok(()),false),Err(QueueError::StaleLease));}
    #[test] fn retryable_failure_returns_job_to_queue_until_attempt_limit(){let mut q=JobQueue::new(2).unwrap();q.submit(job("a")).unwrap();let l=q.lease_next("worker",100,10).unwrap().unwrap();q.finish("a","worker",l.token,101,Err("transient".into()),true).unwrap();assert_eq!(q.get("a").unwrap().state,JobState::Queued);}
    #[test] fn submit_rejects_a_forged_nonqueued_job_state(){let mut q=JobQueue::new(2).unwrap();let mut forged=job("a");forged.state=JobState::Succeeded;assert_eq!(q.submit(forged),Err(QueueError::InvalidJob));assert_eq!(q.iter().count(),0);}
}
