# Stage Manager for Windows — GitHub Wiki Repository

This directory contains the complete source for the **Stage Manager for Windows GitHub Wiki**.

---

## How to Publish to GitHub Wiki

GitHub hosts project wikis in a separate Git repository ending with `.wiki.git`.

### Steps to Sync:
1. Enable the Wiki in your GitHub repository:
   - Navigate to your repository **Settings** → **Features** → check **Wikis**.
2. Clone your repository's wiki to a local folder:
   ```powershell
   git clone https://github.com/fruitsaladwindowsai-zammy/Stage-manager-Rust.wiki.git
   ```
3. Copy all files from `d:\Projects\Stage manager\wiki\*` into the cloned wiki folder:
   ```powershell
   Copy-Item -Recurse -Force "d:\Projects\Stage manager\wiki\*" "Stage-manager-Rust.wiki\"
   ```
4. Commit and push:
   ```powershell
   cd Stage-manager-Rust.wiki
   git add .
   git commit -m "Update GitHub Wiki documentation"
   git push origin master
   ```
5. Your GitHub Wiki will immediately render all chapters, diagrams, and sidebars!
