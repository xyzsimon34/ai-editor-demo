# AI Text Editor

A lightweight, Notion-style AI text editor built on top of the Novel editor suite.

## Installation Steps

1. **Install Dependencies**
   ```bash
   yarn install
   
2. **Set Up Environment Variables**
   
   Create a `.env.local` file ：
   ```env
   OPENAI_API_KEY=your_openai_api_key_here
   ```

3. **Start Development Server**
   ```bash
   yarn dev
   # 或
   npm run dev
   ```

4. **Access the Application**
   
   http://localhost:3000


## Notes

1. API Key: A valid OpenAI API Key is required for AI features to function.
2. Images: Currently uses Base64 encoding for uploads (can be extended to cloud storage like S3 or UploadThing).
3. Data Persistence: Content is stored in the browser's localStorage; clearing browser data will result in content loss.
