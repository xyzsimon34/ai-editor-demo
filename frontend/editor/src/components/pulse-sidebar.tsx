'use client'

import { useState } from 'react'
import { Sparkles, ChevronRight, X } from 'lucide-react'

import type { Agent } from '@/types/ai'

import { Button } from '@/components/base/Button'
import { usePulse } from '@/hooks/usePulse'
import { cn } from '@/lib/utils'

interface PulseSidebarProps {
  editorText: string
  isOpen: boolean
  onToggle: () => void
}

export function PulseSidebar({ editorText, isOpen, onToggle }: PulseSidebarProps) {
  const [selectedAgents, setSelectedAgents] = useState<Agent[]>(['researcher', 'refiner'])

  const { suggestions, isLoading, error, getPulse, reset } = usePulse()

  const handleGetSuggestions = async () => {
    if (!editorText.trim()) return
    await getPulse(editorText, selectedAgents)
  }

  const toggleAgent = (agent: Agent) => {
    setSelectedAgents((prev) =>
      prev.includes(agent) ? prev.filter((a) => a !== agent) : [...prev, agent]
    )
  }

  return (
    <>
      {/* Toggle Button - Fixed Position */}
      {!isOpen && (
        <button
          onClick={onToggle}
          className={cn(
            'fixed bottom-4 right-4 z-50',
            'flex items-center gap-2 rounded-lg px-4 py-3 shadow-lg',
            'bg-primary text-primary-foreground hover:bg-primary/90',
            'transition-all duration-200 hover:scale-105'
          )}
        >
          <Sparkles className={"size-5"} />
          <span className={"font-medium"}>{"AI Suggestions"}</span>
        </button>
      )}

      {/* Sidebar */}
      <div
        className={cn(
          'fixed right-0 top-0 z-40 h-full',
          'w-96 border-l border-border bg-background shadow-xl',
          'transition-transform duration-300 ease-in-out',
          'flex flex-col',
          isOpen ? 'translate-x-0' : 'translate-x-full'
        )}
      >
        {/* Header */}
        <div className={"flex items-center justify-between border-b border-border bg-muted/50 p-4"}>
          <div className={"flex items-center gap-2"}>
            <Sparkles className={"size-5 text-primary"} />
            <h2 className={"font-semibold"}>{"AI Suggestions"}</h2>
          </div>
          <button
            onClick={onToggle}
            className={"rounded-md p-1 transition-colors hover:bg-muted"}
          >
            <X className={"size-4"} />
          </button>
        </div>

        {/* Controls */}
        <div className={"space-y-3 border-b border-border p-4"}>
          <div className={"space-y-2"}>
            <label className={"text-xs font-medium text-muted-foreground"}>
              {"Select Agents"}
            </label>
            <div className={"flex flex-col gap-2"}>
              <label className={"flex cursor-pointer items-center gap-2 rounded-md p-2 transition-colors hover:bg-muted"}>
                <input
                  type={"checkbox"}
                  checked={selectedAgents.includes('researcher')}
                  onChange={() => toggleAgent('researcher')}
                  disabled={isLoading}
                  className={"size-4"}
                />
                <div className={"flex-1"}>
                  <div className={"text-sm font-medium"}>{"Researcher"}</div>
                  <div className={"text-xs text-muted-foreground"}>
                    {"Get context and background info"}
                  </div>
                </div>
              </label>
              <label className={"flex cursor-pointer items-center gap-2 rounded-md p-2 transition-colors hover:bg-muted"}>
                <input
                  type={"checkbox"}
                  checked={selectedAgents.includes('refiner')}
                  onChange={() => toggleAgent('refiner')}
                  disabled={isLoading}
                  className={"size-4"}
                />
                <div className={"flex-1"}>
                  <div className={"text-sm font-medium"}>{"Refiner"}</div>
                  <div className={"text-xs text-muted-foreground"}>
                    {"Polish and improve text quality"}
                  </div>
                </div>
              </label>
            </div>
          </div>

          <Button
            onClick={handleGetSuggestions}
            disabled={isLoading || !editorText.trim() || selectedAgents.length === 0}
            className={"w-full"}
            size={"sm"}
          >
            {isLoading ? (
              <>
                <div className={"mr-2 size-4 animate-spin rounded-full border-2 border-background border-t-transparent"} />
                {"Processing..."}
              </>
            ) : (
              <>
                <Sparkles className={"mr-2 size-4"} />
                {"Get Suggestions"}
              </>
            )}
          </Button>

          {suggestions && (
            <Button
              onClick={reset}
              variant={"outline"}
              size={"sm"}
              className={"w-full"}
            >
              {"Clear Results"}
            </Button>
          )}
        </div>

        {/* Content */}
        <div className={"flex-1 space-y-4 overflow-y-auto p-4"}>
          {/* Empty State */}
          {!suggestions && !error && !isLoading && (
            <div className={"flex h-full flex-col items-center justify-center px-4 text-center"}>
              <Sparkles className={"mb-4 size-12 text-muted-foreground/50"} />
              <p className={"mb-2 text-sm text-muted-foreground"}>
                {"No suggestions yet"}
              </p>
              <p className={"text-xs text-muted-foreground/70"}>
                {"Write something in the editor and click "}&quot;{"Get Suggestions"}&quot;{" to receive AI-powered insights"}
              </p>
            </div>
          )}

          {/* Error State */}
          {error && (
            <div className={"rounded-lg border border-destructive/20 bg-destructive/10 p-4"}>
              <p className={"mb-1 text-sm font-medium text-destructive"}>{"Error"}</p>
              <p className={"text-xs text-destructive/80"}>{error.message}</p>
            </div>
          )}

          {/* Loading State */}
          {isLoading && (
            <div className={"flex flex-col items-center justify-center py-12"}>
              <div className={"mb-4 size-8 animate-spin rounded-full border-2 border-primary border-t-transparent"} />
              <p className={"text-sm text-muted-foreground"}>
                {"Analyzing your content..."}
              </p>
            </div>
          )}

          {/* Suggestions */}
          {suggestions && !isLoading && (
            <div className={"space-y-4"}>
              {Object.entries(suggestions.suggestions).map(([agent, content]) => (
                <div
                  key={agent}
                  className={"overflow-hidden rounded-lg border border-border bg-muted/30"}
                >
                  <div className={"flex items-center gap-2 border-b border-border bg-muted/50 px-3 py-2"}>
                    <div className={"size-2 rounded-full bg-primary"} />
                    <span className={"text-xs font-semibold uppercase tracking-wide text-primary"}>
                      {agent}
                    </span>
                  </div>
                  <div className={"p-3"}>
                    <p className={"whitespace-pre-wrap text-sm leading-relaxed"}>
                      {content}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Overlay */}
      {isOpen && (
        <div
          onClick={onToggle}
          className={"fixed inset-0 z-30 bg-black/20 backdrop-blur-sm transition-opacity duration-300"}
        />
      )}
    </>
  )
}
