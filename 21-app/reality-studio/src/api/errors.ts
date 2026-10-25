/**
 * HELQIX error taxonomy for the frontend.
 * --------------------------------------------------------------------------
 * Every failure surfaced through the UI is normalized into an ApiError with
 * a stable category, so components can present understandable messages while
 * retaining developer diagnostics (status, endpoint, raw detail).
 */

export type ApiErrorCategory =
  | "AUTH_ERROR"
  | "NETWORK_ERROR"
  | "VALIDATION_ERROR"
  | "EXECUTION_ERROR"
  | "STATE_ERROR"
  | "REALITY_ERROR"
  | "BRANCH_ERROR"
  | "VERIFICATION_ERROR"
  | "NOT_FOUND"
  | "BACKEND_UNAVAILABLE"
  | "UNKNOWN_ERROR";

export class ApiError extends Error {
  readonly category: ApiErrorCategory;
  readonly status: number | null;
  readonly endpoint: string | null;
  readonly detail: string | null;

  constructor(
    category: ApiErrorCategory,
    message: string,
    opts: { status?: number; endpoint?: string; detail?: string } = {},
  ) {
    super(message);
    this.name = "ApiError";
    this.category = category;
    this.status = opts.status ?? null;
    this.endpoint = opts.endpoint ?? null;
    this.detail = opts.detail ?? null;
  }

  /** Human-facing sentence — what went wrong and what to do. */
  get userMessage(): string {
    switch (this.category) {
      case "AUTH_ERROR":
        return "Your session is invalid or expired. Please sign in again.";
      case "NETWORK_ERROR":
        return "Could not reach the HELQIX platform services. Check your connection.";
      case "VALIDATION_ERROR":
        return `The request was rejected by validation: ${this.message}`;
      case "EXECUTION_ERROR":
        return `Execution failed: ${this.message}`;
      case "STATE_ERROR":
        return `State access failed: ${this.message}`;
      case "REALITY_ERROR":
        return `Reality operation failed: ${this.message}`;
      case "BRANCH_ERROR":
        return `Branch operation failed: ${this.message}`;
      case "VERIFICATION_ERROR":
        return `Verification failed: ${this.message}`;
      case "NOT_FOUND":
        return "The requested resource does not exist.";
      case "BACKEND_UNAVAILABLE":
        return "This capability is not available from the connected backend yet.";
      case "UNKNOWN_ERROR":
        return `Unexpected error: ${this.message}`;
    }
  }
}

export function categoryFromStatus(status: number): ApiErrorCategory {
  if (status === 401 || status === 403) return "AUTH_ERROR";
  if (status === 404) return "NOT_FOUND";
  if (status === 422 || status === 400) return "VALIDATION_ERROR";
  if (status >= 500) return "UNKNOWN_ERROR";
  return "UNKNOWN_ERROR";
}
