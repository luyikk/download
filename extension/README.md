# DURL Browser Extension
将 Chrome / Edge 的下载请求转发给 **DURL Download Manager** 桌面客户端。
---
## 工作原理
```
用户点击下载链接
      │
      ▼
扩展拦截下载（chrome.downloads.onCreated）
      │
      ├─► 取消浏览器默认下载
      │
      ├─► GET http://127.0.0.1:19283/ping
      │         │
      │         ├─ 200 OK（DURL 正在运行）
      │         │       └─► POST /download { url, cookies, filename }
      │         │                   └─► DURL 弹出「新建下载」对话框（已预填 URL / Cookie）
      │         │
      │         └─ 超时 / 失败（DURL 未运行）
      │                 └─► 恢复浏览器默认下载行为
```
---
## 安装方法（开发者模式）
### Chrome / Edge（通用）
1. 打开扩展管理页面：
   - Chrome：`chrome://extensions/`
   - Edge：`edge://extensions/`
2. 右上角开启 **开发者模式（Developer mode）**
3. 点击 **加载已解压的扩展程序（Load unpacked）**
4. 选择本目录（包含 `manifest.json` 的文件夹）
5. 扩展安装完成 ✅
> 图标文件位于 `icons/` 目录，可替换为真实的 16×16、48×48、128×128 px PNG。
---
## 服务端口
DURL 本地服务默认监听：
```
http://127.0.0.1:19283
```
| 端点 | 方法 | 说明 |
|------|------|------|
| `/ping` | GET | 检测 DURL 是否运行；返回 `200 pong` |
| `/download` | POST | 接收下载请求，请求体为 JSON：`{url, cookies, filename}` |
如需更改端口，同步修改：
- `durl-gui/src/browser_server.rs` 里的 `PORT` 常量
- `extension/background.js` 里的 `APP_BASE` 常量
---
## 跳过特定下载
扩展会跳过以下 URL，不拦截：
- `data:` 开头（内联数据）
- `blob:` 开头（临时对象 URL）
- `chrome-extension:` 开头（扩展内部资源）
---
## 注意事项
- **DURL 未运行时**：扩展自动回退到浏览器原生下载，不会丢失文件。
- **Cookie**：扩展自动抓取目标 URL 的 Cookie 并传给 DURL，方便下载需要登录的文件。
- **隐私**：Cookie 仅传输给本机 `127.0.0.1`，不会发送到任何外部服务器。
