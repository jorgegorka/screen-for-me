// Build the tauri-plugin-updater static manifest (latest.json) for a release.
// Kept pure (no fs/network) so it stays unit-testable.
export function buildLatestJson({ version, pubDate, url, signature }) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(`invalid version "${version}" — expected e.g. 1.3.0 (no leading v)`);
  }
  if (!signature) throw new Error("missing signature (.sig file contents)");
  if (!/^https:\/\//.test(url)) throw new Error(`url must be https: ${url}`);
  return {
    version,
    pub_date: pubDate,
    platforms: {
      "darwin-aarch64": { signature, url },
    },
  };
}
