import { createImageUpload } from "novel";

export const uploadFn = createImageUpload({
  onUpload: async (file: File) => {
    // Convert file to base64 for demo
    return new Promise<string>((resolve) => {
      const reader = new FileReader();
      reader.readAsDataURL(file);
      reader.onload = () => {
        const base64 = reader.result as string;
        resolve(base64);
      };
    });
  },
  validateFn: (file) => {
    if (!file.type.includes("image/")) {
      return false;
    }
    if (file.size / 1024 / 1024 > 20) {
      return false;
    }
    return true;
  },
});
