# Layer 19 — Engineering intent extraction

The current Python module conservatively extracts explicit, unit-bearing values from supported drone, vehicle, and battery briefs. It returns the original source string and character spans for every candidate. If the same field appears with conflicting values, all candidates are retained and marked for review. Unrecognized domains retain the source text and return no structured candidates.

## Current capability

- Payload mass in kilograms
- Battery mass in kilograms
- Target duration in minutes
- Wind speed in metres per second
- Battery capacity in mAh
- Nominal battery voltage
- Mission words such as agriculture, defense, inspection, or mapping
- Vehicle mass, payload mass, battery energy capacity, battery voltage, target range, and target speed for vehicle briefs
- Battery-pack mass and energy capacity, single-cell capacity in Ah or mAh, nominal voltage, discharge current, and cutoff voltage for battery briefs

The authenticated gateway endpoint is `POST /v1/intelligence/intent/parse` with `{"intent":"...","domain":"vehicle"}` (or `drone`). It uses the selected domain and never classifies a product from its name. When `domain` is omitted, the original drone parser is retained for existing clients; new clients should pass an explicit domain. Candidates include exact source spans and remain `needs_review` until the user accepts them. The Reality Studio project-creation form exposes those candidates for review and stores the extraction and review decisions in the project payload.

The default path is deterministic extraction; it does not estimate thrust, endurance, stability, range from capacity, feasibility, or compliance. Unknown domains yield no structured values. An optional AI brief review is available when the gateway is configured with one provider, model, and server-side key. It runs only when an authenticated user explicitly selects **Send brief to {provider} for review**. The provider receives the complete brief. The response is schema-checked, objective proposals must quote an exact substring of the brief, and every proposal stays unaccepted until the user selects it. This assistance is not a component selector, engineering recommendation, or validation engine.

Configure a single provider in the gateway environment with `HELQIX_INTENT_AI_PROVIDER` set to `openai`, `anthropic`, `gemini`, `deepseek`, `kimi`, `qwen`, or `glm`, plus that provider's API key and model variable (`OPENAI_API_KEY`/`OPENAI_MODEL`, `ANTHROPIC_API_KEY`/`ANTHROPIC_MODEL`, `GEMINI_API_KEY`/`GEMINI_MODEL`, `DEEPSEEK_API_KEY`/`DEEPSEEK_MODEL`, `MOONSHOT_API_KEY`/`MOONSHOT_MODEL`, `DASHSCOPE_API_KEY`/`QWEN_MODEL`, or `ZHIPU_API_KEY`/`GLM_MODEL`). Keys remain on the gateway and are never returned to the browser. Qwen currently uses the DashScope international endpoint; deployments in other regions must adjust the endpoint in the provider adapter before use. Without all three settings, AI review is reported as unavailable and the local extractor works as before.

Battery-pack quantities and single-cell quantities use separate fields and are never converted or combined. Cell chemistry, pack topology, thermal behavior, and battery performance remain unknown unless supported by an explicitly selected model and its inputs. Extracted values are candidates for human review, not accepted project specifications.
