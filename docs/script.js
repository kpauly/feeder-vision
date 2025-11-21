const releaseLabel = document.getElementById("release-label");
const windowsBtn = document.getElementById("download-windows");
const macBtn = document.getElementById("download-mac");
const RELEASE_API =
  "https://api.github.com/repos/kpauly/feeder-vision/releases/latest";

async function loadRelease() {
  try {
    const response = await fetch(RELEASE_API, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!response.ok) {
      throw new Error(`Status ${response.status}`);
    }
    const data = await response.json();
    const version = data.tag_name ?? "onbekend";
    releaseLabel.textContent = `Laatste versie: ${version}`;

    const assets = data.assets ?? [];
    const findAsset = (keyword) =>
      assets.find((asset) => asset.name?.toLowerCase().includes(keyword));

    const windowsAsset = findAsset("feediesetup");
    const macAsset = findAsset("feedie-mac");

    if (windowsAsset?.browser_download_url) {
      windowsBtn.href = windowsAsset.browser_download_url;
    }
    if (macAsset?.browser_download_url) {
      macBtn.href = macAsset.browser_download_url;
    }
  } catch (error) {
    releaseLabel.textContent =
      "Laatste versie onbekend – bekijk de releases op GitHub.";
    console.warn("Kon release niet laden", error);
  }
}

document.getElementById("current-year").textContent =
  new Date().getFullYear().toString();

loadRelease();
