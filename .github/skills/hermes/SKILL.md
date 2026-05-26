---
name: durl-downloader
description: >
  Download files from HTTP/HTTPS URLs using the durl CLI tool.
  Use this skill whenever a task requires downloading a file,
  including URLs with no filename, authenticated endpoints, and large files.
agent: hermes
version: "1.0"
---
# Skill: durl-downloader (Hermes Agent)
## Purpose
Invoke the durl command-line tool to download files from HTTP/HTTPS.
durl handles parallel multi-part downloads, streaming when size is unknown,
URL-encoded filenames, and cookie-based authentication.
## Command Pattern
```
durl --url "<URL>" [--save-path "<PATH>"] [--name "<FILENAME>"] [--tasks <N>] [--cookies '<JSON>']
```
## Parameters
| Parameter       | Short | Required | Default | Description |
|-----------------|-------|----------|---------|-------------|
| --url           | -u    | YES      | —       | HTTP/HTTPS download URL |
| --save-path     | -s    | no       | ./      | Directory or full file path |
| --name          | -n    | no       | auto    | Override output filename |
| --tasks         | -t    | no       | 15      | Parallel task count (1-64) |
| --cookies       | -c    | no       | none    | JSON cookies for auth |
## Cookie Format
Object:  {"session":"TOKEN","user":"abc"}
Array:   [{"name":"session","value":"TOKEN"},{"name":"user","value":"abc"}]
## Decision Tree
  URL has visible filename in path?
    YES -> no --name needed
    NO  -> URL has query param fn=, fin=, filename=?
             YES -> durl resolves automatically, no --name needed
             NO  -> provide --name "output.ext"
  Download needs auth?
    YES -> add --cookies '{"key":"value"}'
    NO  -> omit
  File > 100 MB?
    YES -> use --tasks 20 or higher
    NO  -> default 15 is fine
  save-path is a directory?
    YES -> durl appends resolved filename
    NO  -> used as exact output path
## Examples
### Public file
```bash
durl -u "https://example.com/release/app-v1.0.zip"
```
### Save to directory
```bash
durl -u "https://example.com/release/app-v1.0.zip" -s "/tmp/downloads"
```
### No filename in URL
```bash
durl -u "https://api.example.com/export?format=csv" -n "data_export.csv"
```
### URL-encoded filename (auto-resolved)
```bash
# fn=PS4+slim%E6%89%8B%E6%9F%84.zip -> PS4 slim手柄.zip
durl -u "https://baidupcs.example.com/file/...&fn=PS4+slim%E6%89%8B%E6%9F%84.zip&..."
```
### Authenticated download (object cookies)
```bash
durl -u "https://private.example.com/file" \
     -s "/tmp" \
     -n "private.zip" \
     -c '{"session":"SESSION_TOKEN","csrf":"CSRF_TOKEN"}'
```
### Authenticated download (array cookies)
```bash
durl -u "https://private.example.com/file" \
     -c '[{"name":"session","value":"TOKEN"},{"name":"user_id","value":"123"}]'
```
### Large file with high concurrency
```bash
durl -u "https://mirror.example.com/ubuntu-24.04.iso" -s "/isos" -t 30
```
## Output
Success (known size):
  ⠸ [00:00:12] [████████████████████████] 45.0 MiB/45.0 MiB (8.2 MiB/s, ETA 0s)
  done
  [INFO durl] saved to: D:\Downloads\app-v1.0.zip
Success (streaming, unknown size):
  ⠸ [00:00:08] 23.1 MiB downloaded (6.5 MiB/s)
  done
  [INFO durl] saved to: D:\Downloads\data.csv
Error:
  failed
  [ERROR durl] download error: http error:403 Forbidden
## Error Reference
| Error                      | Cause                    | Fix |
|----------------------------|--------------------------|-----|
| http error:403 Forbidden   | Auth required            | Add --cookies with valid session |
| http error:404 Not Found   | URL wrong or expired     | Verify URL |
| http error:401 Unauthorized| Cookie expired           | Refresh credentials |
| down file fail: builder    | Malformed URL            | Check URL syntax |
| io error                   | Disk full or no perms    | Check disk + path permissions |
## Hermes Tool JSON Definition
```json
{
  "name": "durl_download",
  "description": "Download a file from HTTP/HTTPS using durl. Supports parallel downloads, streaming, automatic filename detection, and cookie auth.",
  "parameters": {
    "type": "object",
    "properties": {
      "url": {
        "type": "string",
        "description": "HTTP/HTTPS URL to download"
      },
      "save_path": {
        "type": "string",
        "description": "Directory or full file path. Default: ./",
        "default": "./"
      },
      "filename": {
        "type": "string",
        "description": "Override output filename. Omit to let durl auto-detect."
      },
      "tasks": {
        "type": "integer",
        "description": "Parallel task count. Default 15. Higher = faster for large files.",
        "default": 15,
        "minimum": 1,
        "maximum": 64
      },
      "cookies": {
        "type": "string",
        "description": "JSON cookies. Object {\"k\":\"v\"} or array [{\"name\":\"k\",\"value\":\"v\"}]"
      }
    },
    "required": ["url"]
  }
}
```
## Python Executor
```python
import subprocess
def durl_download(url: str, save_path: str = "./", filename: str = None,
                  tasks: int = 15, cookies: str = None) -> dict:
    cmd = ["durl", "--url", url, "--save-path", save_path, "--tasks", str(tasks)]
    if filename:
        cmd += ["--name", filename]
    if cookies:
        cmd += ["--cookies", cookies]
    result = subprocess.run(cmd, capture_output=True, text=True)
    return {
        "success": result.returncode == 0,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "returncode": result.returncode,
    }
```
## Limitations
- HTTP/HTTPS only (no FTP, SFTP, magnet)
- No resume across process restarts (stale .dd file is deleted on start)
- Cookies sent to all requests including retries