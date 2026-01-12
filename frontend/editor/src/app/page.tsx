import Editor from "@/components/editor";

export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center gap-4 py-4 sm:px-5">
      <div className="flex w-full max-w-screen-lg items-center justify-center px-4 sm:mb-[calc(20vh)]">
        <h1 className="text-2xl font-bold">AI Text Editor Demo</h1>
      </div>
      <Editor />
    </main>
  );
}
