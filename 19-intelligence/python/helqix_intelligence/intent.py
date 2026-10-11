"""Conservative extraction of explicitly stated drone requirements.

This parser does not infer design choices, component selections, feasibility,
or performance. Every returned value points to the exact text that supplied it.
"""

from __future__ import annotations

import re
import json
import os
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import asdict, dataclass
from typing import Any


PARSER_VERSION = "drone-intent-explicit-v1"
NUMBER = r"(?P<value>\d+(?:[.,]\d+)?)"

_PROVIDER_ENV = {
    "openai": ("OPENAI_API_KEY", "https://api.openai.com/v1/chat/completions", "OPENAI_MODEL"),
    "anthropic": ("ANTHROPIC_API_KEY", "https://api.anthropic.com/v1/messages", "ANTHROPIC_MODEL"),
    "gemini": ("GEMINI_API_KEY", "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent", "GEMINI_MODEL"),
    "deepseek": ("DEEPSEEK_API_KEY", "https://api.deepseek.com/chat/completions", "DEEPSEEK_MODEL"),
    "kimi": ("MOONSHOT_API_KEY", "https://api.moonshot.ai/v1/chat/completions", "MOONSHOT_MODEL"),
    "qwen": ("DASHSCOPE_API_KEY", "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions", "QWEN_MODEL"),
    "glm": ("ZHIPU_API_KEY", "https://open.bigmodel.cn/api/paas/v4/chat/completions", "GLM_MODEL"),
}


def ai_review_provider_status() -> dict[str, Any]:
    """Describe server-side AI review configuration without exposing credentials."""
    provider = os.environ.get("HELQIX_INTENT_AI_PROVIDER", "").strip().lower()
    config = _PROVIDER_ENV.get(provider)
    model = os.environ.get(config[2], "").strip() if config else ""
    configured = bool(config and model and os.environ.get(config[0], "").strip())
    return {
        "available": configured,
        "provider": provider if config else None,
        "model": model if configured else None,
        "note": "The brief is sent to the configured provider only when AI review is explicitly requested." if configured
                else "No AI review provider is configured. Local explicit-value review remains available.",
    }


def _provider_request(provider: str, model: str, api_key: str, prompt: str, source_text: str) -> dict[str, Any]:
    """Call the configured provider with a bounded request and return decoded JSON."""
    config = _PROVIDER_ENV[provider]
    endpoint = config[1].format(model=urllib.parse.quote(model, safe="-_."))
    schema = {
        "type": "object", "additionalProperties": False,
        "properties": {
            "objectives": {"type": "array", "items": {"type": "object", "additionalProperties": False,
                "properties": {"statement": {"type": "string"}, "source_quote": {"type": "string"}},
                "required": ["statement", "source_quote"]}},
            "questions": {"type": "array", "items": {"type": "object", "additionalProperties": False,
                "properties": {"question": {"type": "string"}, "reason": {"type": "string"}},
                "required": ["question", "reason"]}},
        }, "required": ["objectives", "questions"],
    }
    if provider == "anthropic":
        body = {"model": model, "max_tokens": 1400,
                "system": "Review engineering briefs as a cautious assistant. Return only source-grounded objective proposals and clarification questions. Never invent numeric values, component selections, performance, safety, feasibility, or compliance. Every objective must include an exact verbatim source_quote. Output must conform to the tool schema.",
                "tools": [{"name": "brief_review", "description": "Return review-only proposals grounded in exact source text.", "input_schema": schema}],
                "tool_choice": {"type": "tool", "name": "brief_review"},
                "messages": [{"role": "user", "content": prompt}]}
        headers = {"x-api-key": api_key, "anthropic-version": "2023-06-01", "content-type": "application/json"}
    elif provider == "gemini":
        body = {"systemInstruction": {"parts": [{"text": "Review engineering briefs cautiously. Never invent numeric values, component selections, performance, safety, feasibility, or compliance. Objective proposals require an exact verbatim source_quote."}]},
                "contents": [{"role": "user", "parts": [{"text": prompt}]}],
                "generationConfig": {"responseMimeType": "application/json", "responseJsonSchema": schema, "maxOutputTokens": 1400}}
        headers = {"x-goog-api-key": api_key, "content-type": "application/json"}
    else:
        body = {"model": model, "temperature": 0, "max_tokens": 1400,
                "response_format": {"type": "json_object"},
                "messages": [{"role": "system", "content": "Review engineering briefs cautiously. Return JSON with objectives and questions. Never invent numeric values, component selections, performance, safety, feasibility, or compliance. Every objective must have an exact verbatim source_quote. No extra keys."},
                             {"role": "user", "content": prompt}]}
        headers = {"Authorization": f"Bearer {api_key}", "content-type": "application/json"}
    request = urllib.request.Request(endpoint, data=json.dumps(body).encode("utf-8"), headers=headers, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.read(1_000_001)
    except urllib.error.HTTPError as exc:
        raise RuntimeError(f"Configured {provider} provider returned HTTP {exc.code}.") from None
    except (urllib.error.URLError, TimeoutError) as exc:
        raise RuntimeError(f"Configured {provider} provider could not be reached.") from None
    if len(raw) > 1_000_000:
        raise RuntimeError("AI provider response exceeded the 1 MB limit.")
    try:
        response_data = json.loads(raw)
        if provider == "anthropic":
            result = next(item["input"] for item in response_data["content"] if item.get("type") == "tool_use" and item.get("name") == "brief_review")
        elif provider == "gemini":
            result = json.loads(response_data["candidates"][0]["content"]["parts"][0]["text"])
        else:
            result = json.loads(response_data["choices"][0]["message"]["content"])
    except (KeyError, IndexError, StopIteration, TypeError, json.JSONDecodeError):
        raise RuntimeError("AI provider returned a response that could not be read as the required structured review.") from None
    return _validate_ai_review(result, source_text)


def _validate_ai_review(result: Any, source_text: str) -> dict[str, Any]:
    if not isinstance(result, dict) or set(result) != {"objectives", "questions"}:
        raise RuntimeError("AI review did not match the required response shape.")
    objectives = result["objectives"]
    questions = result["questions"]
    if not isinstance(objectives, list) or not isinstance(questions, list) or len(objectives) > 20 or len(questions) > 20:
        raise RuntimeError("AI review contained too many or invalid proposals.")
    accepted_objectives = []
    for item in objectives:
        if not isinstance(item, dict) or set(item) != {"statement", "source_quote"}:
            raise RuntimeError("AI review contained an invalid objective proposal.")
        statement, quote = item["statement"], item["source_quote"]
        if not isinstance(statement, str) or not isinstance(quote, str) or not statement.strip() or not quote or quote not in source_text or len(statement) > 500 or len(quote) > 1000:
            raise RuntimeError("AI review included an objective without an exact source quote.")
        accepted_objectives.append({"statement": statement.strip(), "source_quote": quote, "review_status": "needs_review"})
    accepted_questions = []
    for item in questions:
        if not isinstance(item, dict) or set(item) != {"question", "reason"}:
            raise RuntimeError("AI review contained an invalid clarification question.")
        question, reason = item["question"], item["reason"]
        if not isinstance(question, str) or not isinstance(reason, str) or not question.strip() or not reason.strip() or len(question) > 500 or len(reason) > 500:
            raise RuntimeError("AI review contained an invalid clarification question.")
        accepted_questions.append({"question": question.strip(), "reason": reason.strip()})
    return {"objectives": accepted_objectives, "questions": accepted_questions, "review_required": True}


def review_engineering_intent_with_ai(text: str) -> dict[str, Any]:
    """Request optional provider assistance; all output is untrusted and review-only."""
    if not text.strip():
        raise ValueError("intent must not be empty")
    if len(text) > 5000:
        raise ValueError("intent must be 5000 characters or fewer")
    status = ai_review_provider_status()
    provider = status["provider"]
    config = _PROVIDER_ENV.get(provider)
    if not status["available"] or not config:
        raise RuntimeError("AI review is unavailable because no provider, model, and server-side API key are configured.")
    prompt = ("Review the following user-authored engineering brief. Extract only clearly stated objectives, each tied to an exact substring. "
              "Ask concise questions for important missing details. Do not answer the questions or assume defaults. Return an empty list when unsupported.\n\n"
              "<engineering_brief>\n" + text + "\n</engineering_brief>")
    result = _provider_request(provider, status["model"], os.environ[config[0]].strip(), prompt, text)
    result["provider"] = provider
    result["model"] = status["model"]
    result["provenance"] = "external_model_proposal"
    result["review_required"] = True
    return result


@dataclass(frozen=True)
class RequirementCandidate:
    key: str
    label: str
    value: float | str
    unit: str | None
    source_text: str
    start: int
    end: int
    confidence: str = "explicit"
    review_status: str = "needs_review"


PATTERNS: tuple[tuple[str, str, str | None, tuple[str, ...]], ...] = (
    ("payload_mass_kg", "Payload mass", "kg", (
        rf"\b{NUMBER}\s*(?:kg|kilograms?)\s+(?:payload|payload mass)\b",
        rf"\b(?:payload|payload mass)(?:\s+(?:of|up to|at least))?\s*[:=]?\s*{NUMBER}\s*(?:kg|kilograms?)\b",
        rf"\b(?:carry|carries|carrying|lift|lifting)\s+(?:a\s+)?{NUMBER}\s*(?:kg|kilograms?)(?:\s+(?:payload|load))?\b",
    )),
    ("battery_mass_kg", "Battery mass", "kg", (
        rf"\b{NUMBER}\s*(?:kg|kilograms?)\s+(?:battery|battery pack)\b",
        rf"\b(?:battery|battery pack)\s+(?:mass|weight)(?:\s+of)?\s*[:=]?\s*{NUMBER}\s*(?:kg|kilograms?)\b",
    )),
    ("target_duration_min", "Target mission duration", "min", (
        rf"\b{NUMBER}\s*(?:minutes?|mins?)\b",
    )),
    ("wind_speed_m_s", "Wind speed", "m/s", (
        rf"\b{NUMBER}\s*(?:m/s|mps)\s+(?:wind|crosswind)\b",
        rf"\b(?:wind|crosswind)(?:\s+(?:speed|up to|of))?\s*[:=]?\s*{NUMBER}\s*(?:m/s|mps)\b",
    )),
    ("battery_capacity_mah", "Battery capacity", "mAh", (
        rf"\b{NUMBER}\s*mAh\b",
    )),
    ("battery_voltage_v", "Battery nominal voltage", "V", (
        rf"\b{NUMBER}\s*(?:V|volts?)\s+(?:battery|nominal)\b",
        rf"\b(?:battery|nominal voltage)(?:\s+(?:voltage|of))?\s*[:=]?\s*{NUMBER}\s*(?:V|volts?)\b",
    )),
)

VEHICLE_PATTERNS: tuple[tuple[str, str, str | None, tuple[str, ...]], ...] = (
    ("vehicle_mass_kg", "Vehicle mass", "kg", (
        rf"\b(?:vehicle|running|curb|kerb|gross)\s+(?:mass|weight)(?:\s+(?:of|is))?\s*[:=]?\s*{NUMBER}\s*(?:kg|kilograms?)\b",
        rf"\b{NUMBER}\s*(?:kg|kilograms?)\s+(?:vehicle|car|truck)\b",
        rf"\b(?:vehicle|car|truck)\s+weighs?\s*[:=]?\s*{NUMBER}\s*(?:kg|kilograms?)\b",
    )),
    ("payload_mass_kg", "Payload mass", "kg", (
        rf"\b(?:payload|payload mass)\s*(?:of|is|up to|at least)?\s*[:=]?\s*{NUMBER}\s*(?:kg|kilograms?)\b",
        rf"\b{NUMBER}\s*(?:kg|kilograms?)\s+(?:payload|cargo)\b",
    )),
    ("battery_capacity_kwh", "Battery energy capacity", "kWh", (
        rf"\b(?:battery|pack)(?:\s+(?:energy|capacity))?\s*(?:of|is|rated at)?\s*[:=]?\s*{NUMBER}\s*kwh\b",
        rf"\b{NUMBER}\s*kwh\s+(?:battery|battery pack|pack)\b",
    )),
    ("battery_voltage_v", "Battery voltage", "V", (
        rf"\b(?:battery|pack)(?:\s+(?:nominal\s+)?voltage)?\s*(?:of|is|rated at)?\s*[:=]?\s*{NUMBER}\s*(?:v|volts?)\b",
        rf"\b{NUMBER}\s*(?:v|volts?)\s+(?:battery|battery pack|pack)\b",
    )),
    ("target_range_km", "Target range", "km", (
        rf"\b(?:target\s+)?range(?:\s+of|\s+at least|\s+up to)?\s*[:=]?\s*{NUMBER}\s*(?:km|kilometers?|kilometres?)\b",
        rf"\b{NUMBER}\s*(?:km|kilometers?|kilometres?)\s+(?:range|range target)\b",
    )),
    ("target_speed_kmh", "Target speed", "km/h", (
        rf"\b(?:top|maximum|max|target|cruising)\s+speed\s*(?:of|is)?\s*[:=]?\s*{NUMBER}\s*(?:km/h|kph)\b",
        rf"\b{NUMBER}\s*(?:km/h|kph)\s+(?:top\s+)?speed\b",
    )),
)

BATTERY_PATTERNS: tuple[tuple[str, str, str | None, tuple[str, ...]], ...] = (
    ("battery_mass_kg", "Battery mass", "kg", (
        rf"\b(?:battery|battery pack|battery module)\s+(?:mass|weight)(?:\s+(?:of|is))?\s*[:=]?\s*{NUMBER}\s*(?:kg|kilograms?)\b",
        rf"\b{NUMBER}\s*(?:kg|kilograms?)\s+(?:battery|battery pack|battery module)\b",
    )),
    ("battery_energy_capacity_kwh", "Battery energy capacity", "kWh", (
        rf"\b(?:battery|pack)\s+(?:energy\s+)?capacity\s*(?:of|is|rated at)?\s*[:=]?\s*{NUMBER}\s*kwh\b",
        rf"\b{NUMBER}\s*kwh\s+(?:battery|battery pack|pack)\b",
    )),
    ("battery_cell_capacity_ah", "Cell capacity", "Ah", (
        rf"\b(?:cell\s+)?capacity\s*(?:of|is|rated at)?\s*[:=]?\s*{NUMBER}\s*ah\b",
        rf"\b{NUMBER}\s*ah\s+(?:cell\s+)?capacity\b",
    )),
    ("battery_cell_capacity_mah", "Cell capacity", "mAh", (
        rf"\b(?:cell\s+)?capacity\s*(?:of|is|rated at)?\s*[:=]?\s*{NUMBER}\s*mah\b",
        rf"\b{NUMBER}\s*mah\s+(?:cell\s+)?capacity\b",
    )),
    ("battery_nominal_voltage_v", "Battery nominal voltage", "V", (
        rf"\b(?:battery|pack|cell)?\s*nominal\s+voltage\s*(?:of|is|rated at)?\s*[:=]?\s*{NUMBER}\s*(?:v|volts?)\b",
        rf"\b{NUMBER}\s*(?:v|volts?)\s+nominal\s+(?:battery|pack|cell)\s+voltage\b",
    )),
    ("battery_discharge_current_a", "Discharge current", "A", (
        rf"\b(?:discharge|discharging)\s+current\s*(?:of|is|at)?\s*[:=]?\s*{NUMBER}\s*(?:a|amps?)\b",
        rf"\b{NUMBER}\s*(?:a|amps?)\s+(?:discharge|discharging)\s+current\b",
    )),
    ("battery_cutoff_voltage_v", "Cutoff voltage", "V", (
        rf"\b(?:cutoff|cut-off|minimum)\s+voltage\s*(?:of|is|at)?\s*[:=]?\s*{NUMBER}\s*(?:v|volts?)\b",
        rf"\b{NUMBER}\s*(?:v|volts?)\s+(?:cutoff|cut-off|minimum)\s+voltage\b",
    )),
)


def _number(raw: str) -> float:
    return float(raw.replace(",", "."))


def _find_candidates(text: str) -> list[RequirementCandidate]:
    candidates: list[RequirementCandidate] = []
    for key, label, unit, alternatives in PATTERNS:
        found: list[RequirementCandidate] = []
        for pattern in alternatives:
            for match in re.finditer(pattern, text, flags=re.IGNORECASE):
                start, end = match.span()
                if any(start < item.end and end > item.start for item in found):
                    continue
                found.append(RequirementCandidate(
                    key=key, label=label, value=_number(match.group("value")), unit=unit,
                    source_text=match.group(0), start=start, end=end,
                ))
        # Do not silently choose between contradictory repeated requirements.
        values = {candidate.value for candidate in found}
        if len(values) == 1 and found:
            candidates.append(found[0])
        elif len(values) > 1:
            for candidate in found:
                candidates.append(RequirementCandidate(**{**asdict(candidate), "review_status": "conflict_review_required"}))

    mission = None
    for label, words in (
        ("agriculture", ("agriculture", "agricultural", "crop spraying", "crop monitoring")),
        ("defense", ("defense", "defence", "military")),
        ("inspection", ("inspection", "inspect")),
        ("mapping", ("mapping", "survey", "surveying")),
    ):
        for word in words:
            match = re.search(re.escape(word), text, flags=re.IGNORECASE)
            if match:
                mission = RequirementCandidate(
                    key="mission_type", label="Mission type", value=label, unit=None,
                    source_text=match.group(0), start=match.start(), end=match.end(),
                )
                break
        if mission:
            break
    if mission:
        candidates.append(mission)

    return sorted(candidates, key=lambda item: (item.start, item.end, item.key))


def parse_drone_intent(text: str) -> dict[str, Any]:
    """Return reviewable candidates; never returns computed or inferred values."""
    if not text.strip():
        raise ValueError("intent must not be empty")
    if len(text) > 5000:
        raise ValueError("intent must be 5000 characters or fewer")

    candidates = _find_candidates(text)
    by_key: dict[str, list[dict[str, Any]]] = {}
    for candidate in candidates:
        by_key.setdefault(candidate.key, []).append(asdict(candidate))
    return {
        "parser_version": PARSER_VERSION,
        "method": "explicit_unit_pattern_extraction",
        "source_text": text,
        "candidates": [asdict(candidate) for candidate in candidates],
        "conflicts": [key for key, values in by_key.items() if len({item["value"] for item in values}) > 1],
        "not_inferred": ["motor model", "propeller model", "thrust", "flight time", "stability", "regulatory compliance"],
        "review_required": True,
    }


def _parse_with_patterns(text: str, patterns: tuple[tuple[str, str, str | None, tuple[str, ...]], ...],
                         parser_version: str, scope: str, not_inferred: list[str]) -> dict[str, Any]:
    candidates: list[RequirementCandidate] = []
    for key, label, unit, alternatives in patterns:
        found: list[RequirementCandidate] = []
        for pattern in alternatives:
            for match in re.finditer(pattern, text, flags=re.IGNORECASE):
                start, end = match.span()
                if any(start < item.end and end > item.start for item in found):
                    continue
                found.append(RequirementCandidate(
                    key=key, label=label, value=_number(match.group("value")), unit=unit,
                    source_text=match.group(0), start=start, end=end,
                ))
        if len({candidate.value for candidate in found}) == 1 and found:
            candidates.append(found[0])
        elif len({candidate.value for candidate in found}) > 1:
            candidates.extend(RequirementCandidate(**{**asdict(item), "review_status": "conflict_review_required"}) for item in found)

    by_key: dict[str, list[dict[str, Any]]] = {}
    for candidate in candidates:
        by_key.setdefault(candidate.key, []).append(asdict(candidate))
    return {
        "parser_version": parser_version,
        "method": "explicit_unit_pattern_extraction",
        "scope": scope,
        "source_text": text,
        "candidates": [asdict(candidate) for candidate in sorted(candidates, key=lambda item: (item.start, item.end, item.key))],
        "conflicts": [key for key, values in by_key.items() if len({item["value"] for item in values}) > 1],
        "not_inferred": not_inferred,
        "review_required": True,
    }


def _normalized_domain(domain: str | None) -> str:
    value = re.sub(r"[^a-z0-9]+", "_", (domain or "").strip().lower()).strip("_")
    tokens = set(value.split("_"))
    if tokens.intersection({"drone", "uav", "quadrotor", "aircraft"}):
        return "drone"
    if tokens.intersection({"vehicle", "automotive", "automobile", "car", "truck", "ev"}) or "electric_car" in value:
        return "vehicle"
    if tokens.intersection({"battery", "cell", "energy", "storage"}):
        return "battery"
    return "general"


def parse_engineering_intent(text: str, domain: str | None = None) -> dict[str, Any]:
    """Extract only explicitly stated supported quantities for a selected domain.

    Unknown domains return the source with no candidates. The parser never
    classifies a design or estimates missing engineering values.
    """
    if not text.strip():
        raise ValueError("intent must not be empty")
    if len(text) > 5000:
        raise ValueError("intent must be 5000 characters or fewer")
    # Omitted domain preserves the original endpoint contract for existing
    # drone clients. New clients should always send their selected domain.
    selected_domain = "drone" if domain is None else _normalized_domain(domain)
    if selected_domain == "drone":
        result = parse_drone_intent(text)
        result["scope"] = "drone design brief; explicit candidates only"
        return result
    if selected_domain == "vehicle":
        return _parse_with_patterns(
            text, VEHICLE_PATTERNS, "vehicle-intent-explicit-v1",
            "vehicle study brief; explicit candidates only",
            ["vehicle performance", "energy consumption", "range from battery capacity", "safety or regulatory compliance", "design feasibility"],
        )
    if selected_domain == "battery":
        return _parse_with_patterns(
            text, BATTERY_PATTERNS, "battery-intent-explicit-v1",
            "battery design brief; explicit candidates only; cell and pack quantities remain distinct",
            ["cell chemistry", "cell-to-pack topology", "state of health", "thermal behavior", "pack performance", "safety or regulatory compliance", "design feasibility"],
        )
    return {
        "parser_version": "engineering-intent-source-only-v1",
        "method": "source_preservation_no_domain_rules",
        "scope": "source-only; no supported field rules for the selected product type",
        "source_text": text,
        "candidates": [], "conflicts": [],
        "not_inferred": ["component choices", "dimensions", "materials", "performance", "feasibility", "compliance"],
        "review_required": True,
    }
