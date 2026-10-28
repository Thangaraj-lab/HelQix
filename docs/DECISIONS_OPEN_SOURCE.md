# Open-source decisions

Recorded on 2026-10-28 on the founder's instruction: "use free and open souce only do it automatically i give permission".

| Topic | Decision | Notes |
|---|---|---|
| Licence of this repository | MIT (root `LICENSE`, all manifests) | Copyright holder text is "HELQIX contributors"; the founder may change it. |
| Modelica / simulation | OpenModelica is not used | FMPy and the Modelica Reference FMUs stay the route. |
| AI provider | Local Ollama (MIT) with Qwen2.5-7B-Instruct (Apache-2.0) | `OllamaProvider` in `helqix-agent`: loopback only, cost 0, replies are requests that still pass the permission ladder and approvals. |
| Evidence signing | Self-held cosign key, kept outside the repo, no Rekor | Only the public key may be committed. Not generated yet: the key must be created by the founder on their machine. |

Not decided by this record: real rig data (gate 7A), the GPU latency run, and any merge. Those stay with the founder.
