import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { navigate, useRoute } from "@/lib/router";

describe("router", () => {
  afterEach(() => {
    window.location.hash = "";
  });

  it("defaults to the home route", () => {
    window.location.hash = "";
    const { result } = renderHook(() => useRoute());
    expect(result.current.name).toBe("home");
  });

  it("resolves the gallery route from the hash", () => {
    window.location.hash = "#/kitchen-sink";
    const { result } = renderHook(() => useRoute());
    expect(result.current.name).toBe("kitchen-sink");
  });

  it("reacts to hash changes", () => {
    window.location.hash = "";
    const { result } = renderHook(() => useRoute());

    act(() => {
      navigate("/kitchen-sink");
      window.dispatchEvent(new HashChangeEvent("hashchange"));
    });

    expect(result.current.name).toBe("kitchen-sink");
  });

  it("treats unknown paths as home rather than a blank screen", () => {
    window.location.hash = "#/does-not-exist";
    const { result } = renderHook(() => useRoute());
    expect(result.current.name).toBe("home");
  });
});
