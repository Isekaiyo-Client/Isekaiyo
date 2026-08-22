import { describe, expect, it } from "vitest";
import {
  labelForLoader,
  loaderNeedsVersion,
  MAX_INSTANCE_NAME_LEN,
  validateInstanceForm,
  type InstanceForm,
} from "./validate";

function valid(): InstanceForm {
  return { name: "My PvP", minecraftVersion: "1.21.x", loaderKind: "vanilla", loaderVersion: "" };
}

describe("validateInstanceForm", () => {
  it("accepts a valid vanilla form", () => {
    expect(validateInstanceForm(valid())).toEqual({});
  });

  it("accepts a valid fabric form with version", () => {
    const form = { ...valid(), loaderKind: "fabric" as const, loaderVersion: "0.16.0" };
    expect(validateInstanceForm(form)).toEqual({});
  });

  it("rejects blank names and overlong names", () => {
    expect(validateInstanceForm({ ...valid(), name: "   " }).name).toBeTruthy();
    expect(validateInstanceForm({ ...valid(), name: "x".repeat(MAX_INSTANCE_NAME_LEN + 1) }).name).toContain(
      String(MAX_INSTANCE_NAME_LEN),
    );
  });

  it("rejects missing minecraft version", () => {
    expect(validateInstanceForm({ ...valid(), minecraftVersion: " " }).minecraftVersion).toBeTruthy();
  });

  it("requires a loader version for non-vanilla loaders only", () => {
    const noVersion = { ...valid(), loaderKind: "forge" as const };
    expect(validateInstanceForm(noVersion).loaderVersion).toContain("Forge");
    // Vanilla must NOT demand a version.
    expect(validateInstanceForm(valid()).loaderVersion).toBeUndefined();
  });
});

describe("helpers", () => {
  it("loaderNeedsVersion mirrors the ikk-core invariant", () => {
    expect(loaderNeedsVersion("vanilla")).toBe(false);
    expect(loaderNeedsVersion("quilt")).toBe(true);
  });

  it("labelForLoader produces human labels", () => {
    expect(labelForLoader("neoforge")).toBe("NeoForge");
  });
});
