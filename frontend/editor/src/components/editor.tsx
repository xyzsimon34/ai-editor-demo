"use client";

import {
  EditorCommand,
  EditorCommandEmpty,
  EditorCommandItem,
  EditorCommandList,
  EditorContent,
  type EditorInstance,
  EditorRoot,
  ImageResizer,
  type JSONContent,
  handleCommandNavigation,
  handleImageDrop,
  handleImagePaste,
} from "novel";
import { useEffect, useState } from "react";
import { useDebouncedCallback } from "use-debounce";
import { defaultExtensions } from "./extensions";
import { slashCommand, suggestionItems } from "./slash-command";
import GenerativeMenuSwitch from "./generative/generative-menu-switch";
import { uploadFn } from "./image-upload";
import { TextButtons } from "./selectors/text-buttons";
import { Separator } from "./ui/separator";

const extensions = [...defaultExtensions, slashCommand];

const defaultEditorContent: JSONContent = {
  type: "doc",
  content: [],
};

interface EditorProps {
  onSaveStatusChange?: (status: string) => void;
}

export default function Editor({ onSaveStatusChange }: EditorProps) {
  const [initialContent, setInitialContent] = useState<null | JSONContent>(null);
  const [_saveStatus, setSaveStatus] = useState("Saved");
  const [charsCount, setCharsCount] = useState<number>();

  const [_openNode, _setOpenNode] = useState(false);
  const [_openColor, _setOpenColor] = useState(false);
  const [_openLink, _setOpenLink] = useState(false);
  const [openAI, setOpenAI] = useState(false);

  const debouncedUpdates = useDebouncedCallback(async (editor: EditorInstance) => {
    const json = editor.getJSON();
    const wordCount = editor.storage.characterCount.words();
    setCharsCount(wordCount > 0 ? wordCount : undefined);
    window.localStorage.setItem("novel-content", JSON.stringify(json));
    window.localStorage.setItem("markdown", editor.storage.markdown.getMarkdown());
    const newStatus = "Saved";
    setSaveStatus(newStatus);
    onSaveStatusChange?.(newStatus);
  }, 500);

  useEffect(() => {
    const content = window.localStorage.getItem("novel-content");
    if (content) setInitialContent(JSON.parse(content));
    else setInitialContent(defaultEditorContent);
  }, []);

  if (!initialContent) return null;

  return (
    <div className={"relative w-full"}>
      <div className={"mb-4 flex items-center justify-end gap-4 text-sm text-muted-foreground"}>
        {charsCount !== undefined && charsCount > 0 && (
          <div className={"flex items-center gap-2"}>
            <span>{charsCount}{" words"}</span>
          </div>
        )}
      </div>
      <EditorRoot>
        <EditorContent
          initialContent={initialContent}
          extensions={extensions}
          className={"relative min-h-[600px] w-full overflow-hidden rounded-lg border border-muted bg-background shadow-sm"}
          editorProps={{
            handleDOMEvents: {
              keydown: (_view, event) => handleCommandNavigation(event),
            },
            handlePaste: (view, event) => handleImagePaste(view, event, uploadFn),
            handleDrop: (view, event, _slice, moved) => handleImageDrop(view, event, moved, uploadFn),
            attributes: {
              class:
                "prose prose-lg dark:prose-invert prose-headings:font-title font-default focus:outline-none max-w-full px-4 sm:px-8 py-6",
            },
          }}
          onUpdate={({ editor }) => {
            debouncedUpdates(editor);
            const newStatus = "Unsaved";
            setSaveStatus(newStatus);
            onSaveStatusChange?.(newStatus);
          }}
          slotAfter={<ImageResizer />}
        >
          <EditorCommand className={"z-50 h-auto max-h-[330px] overflow-y-auto rounded-md border border-muted bg-background px-1 py-2 shadow-md transition-all"}>
            <EditorCommandEmpty className={"px-2 text-muted-foreground"}>{"No results"}</EditorCommandEmpty>
            <EditorCommandList>
              {suggestionItems.map((item) => (
                <EditorCommandItem
                  value={item.title}
                  onCommand={(val) => item.command?.(val)}
                  className={"flex w-full items-center space-x-2 rounded-md px-2 py-1 text-left text-sm hover:bg-accent aria-selected:bg-accent"}
                  key={item.title}
                >
                  <div className={"flex size-10 items-center justify-center rounded-md border border-muted bg-background"}>
                    {item.icon}
                  </div>
                  <div>
                    <p className={"font-medium"}>{item.title}</p>
                    <p className={"text-xs text-muted-foreground"}>{item.description}</p>
                  </div>
                </EditorCommandItem>
              ))}
            </EditorCommandList>
          </EditorCommand>

          <GenerativeMenuSwitch open={openAI} onOpenChange={setOpenAI}>
            <Separator orientation={"vertical"} />
            <TextButtons />
            <Separator orientation={"vertical"} />
          </GenerativeMenuSwitch>
        </EditorContent>
      </EditorRoot>
    </div>
  );
}
