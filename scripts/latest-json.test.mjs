import { describe, expect, it } from "vitest";
import { buildLatestJson } from "./latest-json.mjs";

describe("buildLatestJson", () => {
  const input = {
    version: "1.3.0",
    pubDate: "2026-07-14T12:00:00.000Z",
    url: "https://github.com/jorgegorka/screen-for-me/releases/download/v1.3.0/Screen_for_me_1.3.0_aarch64.app.tar.gz",
    signature: "dW50cnVzdGVkIGNvbW1lbnQ...",
  };

  it("produces Tauri's static manifest shape", () => {
    expect(buildLatestJson(input)).toEqual({
      version: "1.3.0",
      pub_date: "2026-07-14T12:00:00.000Z",
      platforms: {
        "darwin-aarch64": {
          signature: "dW50cnVzdGVkIGNvbW1lbnQ...",
          url: "https://github.com/jorgegorka/screen-for-me/releases/download/v1.3.0/Screen_for_me_1.3.0_aarch64.app.tar.gz",
        },
      },
    });
  });

  it("rejects a malformed version", () => {
    expect(() => buildLatestJson({ ...input, version: "v1.3.0" })).toThrow(/version/);
  });

  it("rejects an empty signature", () => {
    expect(() => buildLatestJson({ ...input, signature: "" })).toThrow(/signature/);
  });

  it("rejects a non-https url", () => {
    expect(() => buildLatestJson({ ...input, url: "http://example.com/x.tar.gz" })).toThrow(/url/);
  });
});
