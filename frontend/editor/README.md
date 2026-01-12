# AI Text Editor

簡易版 AI 文本編輯器，基於 Novel 編輯器。

## 安裝步驟

1. **安裝依賴**
   ```bash
   yarn install
   
2. **設置環境變數**
   
   創建 `.env.local` 文件：
   ```env
   OPENAI_API_KEY=your_openai_api_key_here
   ```

3. **啟動開發服務器**
   ```bash
   yarn dev
   # 或
   npm run dev
   ```

4. **訪問應用**
   
   打開瀏覽器訪問 http://localhost:3000

## 專案結構

```
src/
├── app/
│   ├── api/
│   │   └── generate/      # AI API 路由
│   ├── layout.tsx          # 根布局
│   ├── page.tsx            # 主頁面
│   └── globals.css         # 全局樣式
├── components/
│   ├── editor.tsx          # 主編輯器組件
│   ├── extensions.ts        # TipTap 擴展配置
│   ├── slash-command.tsx    # `/` 命令配置
│   ├── image-upload.ts      # 圖片上傳
│   ├── generative/          # AI 功能組件
│   ├── selectors/           # 格式化選項
│   └── ui/                  # UI 組件庫
└── lib/
    └── utils.ts             # 工具函數
```

## 功能說明

### 基本編輯
- 直接在編輯器中輸入文字
- 支持 Markdown 語法
- 自動保存到 localStorage

### 格式化工具
- **粗體** (Bold)
- **斜體** (Italic)
- **下劃線** (Underline)
- **刪除線** (Strikethrough)
- **行內代碼** (Inline Code)

### AI 功能
選取文字後點擊 "Ask AI" 按鈕，可以：
- **Continue writing** - 繼續寫作
- **Improve writing** - 改進寫作
- **Fix grammar** - 修正語法
- **Make shorter** - 縮短文字
- **Make longer** - 延長文字
- **自定義指令** - 輸入任意指令讓 AI 處理

### 快捷命令
輸入 `/` 開啟命令選單：
- Text - 普通文字
- Heading 1/2/3 - 標題
- Bullet List - 無序列表
- Numbered List - 有序列表
- Quote - 引用
- Code - 代碼塊
- Image - 上傳圖片
- To-do List - 待辦清單

## 技術細節

- **編輯器核心**: TipTap (基於 ProseMirror)
- **AI 集成**: Vercel AI SDK + OpenAI
- **UI 框架**: Next.js 15 + React 18
- **樣式**: TailwindCSS
- **組件庫**: Radix UI

## 注意事項

1. 需要有效的 OpenAI API Key
2. 圖片上傳目前使用 base64 編碼（可擴展為雲存儲）
3. 內容保存在瀏覽器 localStorage，清除瀏覽器數據會丟失
