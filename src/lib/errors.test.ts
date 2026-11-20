import { describe, expect, it } from "vitest";
import { PulseError, toPulseError } from "@/lib/errors";

describe("toPulseError", () => {
  it("returns the same instance when it is already a PulseError", () => {
    const original = new PulseError({
      code: "not_found",
      message: "gone",
      retryable: false,
    });
    expect(toPulseError(original)).toBe(original);
  });

  it("adopts a well-formed backend payload verbatim", () => {
    const error = toPulseError({
      code: "rate_limited",
      message: "slow down",
      retryable: true,
      details: { resetAt: "2026-01-01T00:00:00Z" },
    });

    expect(error).toBeInstanceOf(PulseError);
    expect(error.code).toBe("rate_limited");
    expect(error.retryable).toBe(true);
    expect(error.details).toEqual({ resetAt: "2026-01-01T00:00:00Z" });
  });

  it("rejects a payload that is missing the contract fields", () => {
    const error = toPulseError({ code: "network", message: "timeout" });
    expect(error.code).toBe("unknown");
    expect(error.retryable).toBe(false);
  });

  it("normalises a native Error", () => {
    const error = toPulseError(new Error("boom"));
    expect(error.message).toBe("boom");
    expect(error.code).toBe("unknown");
  });

  it("normalises a thrown string", () => {
    expect(toPulseError("plain string").message).toBe("plain string");
  });

  it("normalises an unrecognised value", () => {
    expect(toPulseError({ weird: true }).message).toBe("Unknown error");
    expect(toPulseError(undefined).message).toBe("Unknown error");
  });
});
