"use client";

import { FileText } from "lucide-react";

interface HeaderProps {
  saveStatus?: string;
  _onSave?: () => void;
}

export default function Header({ saveStatus = "Saved", _onSave }: HeaderProps) {
  const isSaved = saveStatus === "Saved";
  
  return (
    <header className={"sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60"}>
      <div className={"flex h-14 items-center justify-between px-4 sm:px-6 lg:px-8"}>
        <div className={"flex items-center gap-2"}>
          <FileText className={"size-5 text-primary"} />
          <h1 className={"text-lg font-semibold"}>{"AI Editor"}</h1>
        </div>
        
        <div className={"flex items-center gap-4"}>
          <div className={"flex items-center gap-2 text-xs text-muted-foreground"}>
            <div className={`size-2 rounded-full ${isSaved ? "bg-green-500" : "bg-yellow-500"}`}></div>
            <span className={"hidden sm:inline"}>{saveStatus}</span>
          </div>
        </div>
      </div>
    </header>
  );
}
