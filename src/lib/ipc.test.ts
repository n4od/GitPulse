import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { call } from "@/lib/ipc";
import { PulseError } from "@/lib/errors";

describe("call", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("returns the backend value untouched", async () => {
    invoke.mockResolvedValue({ version: "0.1.0" });

    await expect(call("system_info")).resolves.toEqual({ version: "0.1.0" });
    expect(invoke).toHaveBeenCalledWith("system_info", undefined);
  });

  it("forwards arguments", async () => {
    invoke.mockResolvedValue(undefined);

    await call("runs_list", { repoId: 42, limit: 20 });

    expect(invoke).toHaveBeenCalledWith("runs_list", { repoId: 42, limit: 20 });
  });

  it("converts a structured backend error into a PulseError", async () => {
    invoke.mockRejectedValue({
      code: "forbidden",
      message: "the operation failed",
      retryable: false,
    });

    await expect(call("repos_list")).rejects.toBeInstanceOf(PulseError);
    await expect(call("repos_list")).rejects.toMatchObject({
      code: "forbidden",
      retryable: false,
    });
  });

  it("converts an unstructured throw into an unknown PulseError", async () => {
    invoke.mockRejectedValue("transport exploded");

    await expect(call("repos_list")).rejects.toMatchObject({
      code: "unknown",
      message: "transport exploded",
    });
  });
});
