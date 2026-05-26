---
name: durl-downloader
description: >
  Download files from HTTP/HTTPS URLs using the durl CLI tool.
  Register this skill in OpenClaw to enable any agent or workflow node
  to download files with parallel transfer, cookie auth, and smart filename detection.
agent: openclaw
version: "1.0"
---
# Skill: durl-downloader (OpenClaw Agent)
## Overview
This skill wraps the durl binary as an OpenClaw action node.
Returns the absolute path of the downloaded file on success, or an error string on failure.
Capabilities:
- Parallel multi-part HTTP downloads (up to 64 tasks)
- Streaming mode when Content-Length is absent
- Automatic filename from Content-Disposition, URL query params, or URL path
- Cookie injection for authenticated endpoints
- Exponential-backoff retry (10x per block, 300 ms to 5 s)
## OpenClaw Skill Definition
```yaml
skill:
  id: durl.download
  name: "Download File"
  description: >
    Download a file from HTTP/HTTPS. Supports parallel transfer,
    streaming fallback, cookie auth, and automatic filename detection.
  category: io.network
  timeout_seconds: 3600
  inputs:
    - id: url
      type: string
      required: true
      label: "Download URL"
      description: "HTTP or HTTPS URL to download"
    - id: save_path
      type: string
      required: false
      default: "./"
      label: "Save Path"
      description: "Directory or full file path"
    - id: filename
      type: string
      required: false
      label: "Custom Filename"
      description: >
        Override the output filename.
        Leave empty to let durl resolve from Content-Disposition or URL.
    - id: tasks
      type: integer
      required: false
      default: 15
      min: 1
      max: 64
      label: "Parallel Tasks"
    - id: cookies
      type: string
      required: false
      label: "Cookies (JSON)"
      description: >
        Object: {"key":"value"}
        Array:  [{"name":"key","value":"value"}]
  outputs:
    - id: file_path
      type: string
      label: "Saved File Path"
    - id: success
      type: boolean
      label: "Success"
    - id: error
      type: string
      label: "Error Message"
  executor:
    type: shell
    command: |
      durl \
        --url "{{url}}" \
        --save-path "{{save_path}}" \
        --tasks {{tasks}} \
        {% if filename %}--name "{{filename}}"{% endif %} \
        {% if cookies %}--cookies '{{cookies}}'{% endif %}
    success_exit_codes: [0]
    capture_stderr: true
```
## Workflow Node Examples
### Basic download node
```yaml
nodes:
  - id: download_file
    skill: durl.download
    inputs:
      url: "https://example.com/release/app-v2.0.zip"
      save_path: "/workspace/downloads"
    outputs:
      file_path: -> next_node.input_file
```
### Authenticated download with secrets
```yaml
nodes:
  - id: auth_download
    skill: durl.download
    inputs:
      url: "{{ env.DOWNLOAD_URL }}"
      save_path: "{{ env.DOWNLOAD_DIR }}"
      filename: "archive.zip"
      tasks: 20
      cookies: '{"session":"{{ secrets.SESSION_TOKEN }}"}'
    outputs:
      file_path: -> extract_node.archive_path
      success:   -> branch_node.condition
```
### Conditional branch on success/failure
```yaml
nodes:
  - id: download
    skill: durl.download
    inputs:
      url: "{{ inputs.url }}"
      save_path: "/tmp"
  - id: check
    type: branch
    condition: "{{ download.success }}"
    on_true:  -> process_file
    on_false: -> notify_error
  - id: notify_error
    skill: log.error
    inputs:
      message: "Download failed: {{ download.error }}"
```
### URL with encoded filename (auto-decoded)
```yaml
# URL contains fn=PS4+slim%E6%89%8B%E6%9F%84.zip
# durl automatically decodes to: PS4 slim手柄.zip
nodes:
  - id: download
    skill: durl.download
    inputs:
      url: "https://baidupcs.example.com/file/...&fn=PS4+slim%E6%89%8B%E6%9F%84.zip&..."
      save_path: "/downloads"
```
### Baidu PCS with cookie auth
```yaml
nodes:
  - id: download
    skill: durl.download
    inputs:
      url: "{{ baidu_direct_link }}"
      save_path: "{{ workspace }}"
      tasks: 10
      cookies: '{"BDUSS":"{{ secrets.BDUSS }}","BAIDUID":"{{ secrets.BAIDUID }}"}'
```
## Output Reference
| Output    | Type    | Example |
|-----------|---------|---------|
| file_path | string  | /workspace/downloads/app-v2.0.zip |
| success   | boolean | true |
| error     | string  | "" (empty on success) |
## Error Reference
| Error                       | Cause                 | Resolution |
|-----------------------------|-----------------------|------------|
| http error:403 Forbidden    | Auth required         | Add cookies with valid session |
| http error:404 Not Found    | URL wrong or expired  | Verify URL |
| http error:401 Unauthorized | Cookie expired        | Refresh credentials |
| down file fail: builder     | Malformed URL         | Check URL syntax |
| io error                    | Disk full/no perms    | Check disk space and path |
## Prerequisites
durl must be in PATH:
```bash
cargo install durl
# verify:
durl --help
```
## Limitations
- HTTP/HTTPS only (no FTP, SFTP, magnet)
- No resume after process kill (temp .dd file deleted on next start)
- Cookies are sent to all requests for the URL (initial + retries)
- DownloadHandler is single-threaded from caller side; downloads run on internal Tokio pool