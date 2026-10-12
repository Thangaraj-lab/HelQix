import type { EngineeringReality } from '../api';
import type { AuthoritativeEngineeringState } from './authoritativeState';

export const EXTERNAL_EXECUTION_SCHEMA = 'helqix.external-execution.v1' as const;
export type ExternalExecutionStatus = 'planned'|'queued'|'preparing'|'running'|'cancelling'|'completed'|'failed'|'cancelled'|'timed_out';
export interface ExternalToolIdentity { id:string; version:string; executable?:string; provider?:string; }
export interface ExternalExecutionRequest { id:string; tool:ExternalToolIdentity; operation:string; realityRevision:number; inputDigest:string; timeoutMs:number; maxRetries:number; metadata?:Record<string,unknown>; }
export interface ExternalExecutionEvent { sequence:number; timestamp:string; status:ExternalExecutionStatus; message?:string; progress?:number; logicalTimeMs?:number; }
export interface ExternalExecutionResult { status:'completed'|'failed'|'cancelled'|'timed_out'; outputs:Record<string,unknown>; observations:Array<{signal:string;value:unknown;unit?:string;logicalTimeMs?:number}>; state?:AuthoritativeEngineeringState; outputDigest?:string; error?:string; }
export interface ExternalExecutionContext { reality:EngineeringReality; state:AuthoritativeEngineeringState; request:ExternalExecutionRequest; events:ExternalExecutionEvent[]; result?:ExternalExecutionResult; }
export interface ExternalExecutionAdapter { prepare(context:ExternalExecutionContext, signal:AbortSignal):Promise<void>; execute(context:ExternalExecutionContext, signal:AbortSignal):Promise<ExternalExecutionResult>; cancel?(context:ExternalExecutionContext):Promise<void>|void; cleanup?(context:ExternalExecutionContext):Promise<void>|void; }
export interface ExternalExecutionRun { schema:typeof EXTERNAL_EXECUTION_SCHEMA; attempt:number; status:ExternalExecutionStatus; context:ExternalExecutionContext; startedAt?:string; finishedAt?:string; error?:string; }
function now():string{return new Date().toISOString();}
function clone<T>(value:T):T{return structuredClone(value);}
export function createExternalExecution(reality:EngineeringReality,state:AuthoritativeEngineeringState,request:ExternalExecutionRequest):ExternalExecutionRun{
  if(request.realityRevision!==reality.revision)throw new Error('External execution targets a stale engineering reality revision.');
  if(state.realityRevision!==reality.revision)throw new Error('External execution state targets a stale engineering reality revision.');
  if(!Number.isFinite(request.timeoutMs)||request.timeoutMs<=0)throw new Error('External execution timeout must be positive.');
  if(!Number.isInteger(request.maxRetries)||request.maxRetries<0)throw new Error('External execution maxRetries must be a non-negative integer.');
  return{schema:EXTERNAL_EXECUTION_SCHEMA,attempt:0,status:'planned',context:{reality:clone(reality),state:clone(state),request:clone(request),events:[]}};
}
function event(run:ExternalExecutionRun,status:ExternalExecutionStatus,message?:string,progress?:number):ExternalExecutionRun{
  const entry:ExternalExecutionEvent={sequence:run.context.events.length+1,timestamp:now(),status,...(message?{message}:{}),...(progress===undefined?{}:{progress})};
  return{...run,status,context:{...run.context,events:[...run.context.events,entry]}};
}
export async function executeExternal(run:ExternalExecutionRun,adapter:ExternalExecutionAdapter):Promise<ExternalExecutionRun>{
  if(!['planned','queued'].includes(run.status))throw new Error(`Cannot execute external run from ${run.status}.`);
  let active=event({...run,attempt:run.attempt+1,startedAt:now()},'preparing');
  const controller=new AbortController();
  let timeoutHandle:ReturnType<typeof setTimeout>|undefined;
  let timedOut=false;
  let finalRun:ExternalExecutionRun|undefined;
  try{
    const work=(async()=>{await adapter.prepare(active.context,controller.signal);if(controller.signal.aborted)throw new Error('External execution aborted during preparation.');active=event(active,'running','External tool execution started.',0);return adapter.execute(active.context,controller.signal);})();
    const deadline=new Promise<never>((_,reject)=>{timeoutHandle=setTimeout(()=>{timedOut=true;controller.abort();reject(new Error(`External execution exceeded ${active.context.request.timeoutMs}ms.`));},active.context.request.timeoutMs);});
    const result=await Promise.race([work,deadline]);
    if(controller.signal.aborted)throw new Error('External execution aborted.');
    const finalStatus:ExternalExecutionStatus=result.status==='completed'?'completed':result.status==='cancelled'?'cancelled':result.status==='timed_out'?'timed_out':'failed';
    const finished=event(active,finalStatus,result.error??(finalStatus==='completed'?'External execution completed.':'External tool did not complete successfully.'),finalStatus==='completed'?100:undefined);
    finalRun={...finished,context:{...finished.context,result:clone(result)},finishedAt:now(),...(result.error?{error:result.error}:{})};
  }catch(error){
    const message=timedOut?`External execution exceeded ${active.context.request.timeoutMs}ms.`:error instanceof Error?error.message:'External execution failed.';
    controller.abort();
    try{await Promise.resolve(adapter.cancel?.(active.context));}catch{/* preserve primary failure */}
    const status:ExternalExecutionStatus=timedOut?'timed_out':'failed';
    const failed=event(active,status,message);
    finalRun={...failed,context:{...failed.context,result:{status:timedOut?'timed_out':'failed',outputs:{},observations:[],error:message}},finishedAt:now(),error:message};
  }finally{
    if(timeoutHandle!==undefined)clearTimeout(timeoutHandle);
    const cleanupContext=finalRun?.context??active.context;
    try{await Promise.resolve(adapter.cleanup?.(cleanupContext));}catch(error){
      const message=error instanceof Error?error.message:'External adapter cleanup failed.';
      const base=finalRun??active;
      const warning:ExternalExecutionEvent={sequence:base.context.events.length+1,timestamp:now(),status:base.status,message:`Cleanup warning: ${message}`};
      finalRun={...base,context:{...base.context,events:[...base.context.events,warning]}};
    }
  }
  return finalRun!;
}
export function canRetryExternal(run:ExternalExecutionRun):boolean{return['failed','timed_out'].includes(run.status)&&run.attempt<=run.context.request.maxRetries;}
export function retryExternal(run:ExternalExecutionRun):ExternalExecutionRun{if(!canRetryExternal(run))throw new Error('External execution retry budget is exhausted.');return{...run,status:'queued',error:undefined,finishedAt:undefined,context:{...run.context,result:undefined}};}
export function cancelExternal(run:ExternalExecutionRun):ExternalExecutionRun{if(!['planned','queued','preparing','running'].includes(run.status))return run;return event(run,'cancelling','Cancellation requested.');}
