// Pure dashboard formatting helpers.
import { describe, expect, it } from "vitest";
import { abbrev, abbrevU, commas, relativeTime, signed } from "./format";

describe("commas", () => {
  it("groups thousands", () => {
    expect(commas(0)).toBe("0");
    expect(commas(999)).toBe("999");
    expect(commas(1000)).toBe("1,000");
    expect(commas(1234567)).toBe("1,234,567");
  });

  it("keeps the sign and rounds", () => {
    expect(commas(-1234567)).toBe("-1,234,567");
    expect(commas(1499.6)).toBe("1,500");
  });
});

describe("signed", () => {
  it("always shows a leading sign", () => {
    expect(signed(5)).toBe("+5");
    expect(signed(0)).toBe("+0");
    expect(signed(-5)).toBe("-5");
    expect(signed(12000)).toBe("+12,000");
  });
});

describe("abbrev / abbrevU", () => {
  it("abbreviates magnitudes", () => {
    expect(abbrevU(999)).toBe("999");
    expect(abbrevU(2000)).toBe("2k");
    expect(abbrevU(1_500_000)).toBe("1.5M");
  });

  it("prefixes a sign on abbrev", () => {
    expect(abbrev(500)).toBe("+500");
    expect(abbrev(-2000)).toBe("-2k");
    expect(abbrev(1_500_000)).toBe("+1.5M");
    expect(abbrev(-1_200_000)).toBe("-1.2M");
  });
});

describe("relativeTime", () => {
  it("handles missing and invalid input", () => {
    expect(relativeTime(null)).toBe("never");
    expect(relativeTime("not-a-date")).toBe("never");
  });

  it("describes recent times", () => {
    expect(relativeTime(new Date(Date.now() - 1000).toISOString())).toBe("just now");
    expect(relativeTime(new Date(Date.now() - 5 * 60_000).toISOString())).toBe("5m ago");
    expect(relativeTime(new Date(Date.now() - 3 * 3600_000).toISOString())).toBe("3h ago");
    expect(relativeTime(new Date(Date.now() - 2 * 86_400_000).toISOString())).toBe("2d ago");
  });
});
