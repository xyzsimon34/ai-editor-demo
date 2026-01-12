import { openai } from "@ai-sdk/openai";
import { streamText } from "ai";

export const runtime = "edge";

export async function POST(req: Request): Promise<Response> {
  try {
    if (!process.env.OPENAI_API_KEY || process.env.OPENAI_API_KEY === "") {
      return new Response("Missing OPENAI_API_KEY - make sure to add it to your .env.local file.", {
        status: 400,
      });
    }

    const { prompt, option, command } = await req.json();
    
    let messages: Array<{ role: "system" | "user"; content: string }> = [];

    switch (option) {
      case "continue":
        messages = [
          {
            role: "system" as const,
            content:
              "You are an AI writing assistant that continues existing text based on context from prior text. " +
              "Give more weight/priority to the later characters than the beginning ones. " +
              "Limit your response to no more than 200 characters, but make sure to construct complete sentences." +
              "Use Markdown formatting when appropriate.",
          },
          {
            role: "user" as const,
            content: prompt,
          },
        ];
        break;
      case "improve":
        messages = [
          {
            role: "system" as const,
            content:
              "You are an AI writing assistant that improves existing text. " +
              "Limit your response to no more than 200 characters, but make sure to construct complete sentences." +
              "Use Markdown formatting when appropriate.",
          },
          {
            role: "user" as const,
            content: `The existing text is: ${prompt}`,
          },
        ];
        break;
      case "shorter":
        messages = [
          {
            role: "system" as const,
            content:
              "You are an AI writing assistant that shortens existing text. " +
              "Use Markdown formatting when appropriate.",
          },
          {
            role: "user" as const,
            content: `The existing text is: ${prompt}`,
          },
        ];
        break;
      case "longer":
        messages = [
          {
            role: "system" as const,
            content:
              "You are an AI writing assistant that lengthens existing text. " +
              "Use Markdown formatting when appropriate.",
          },
          {
            role: "user" as const,
            content: `The existing text is: ${prompt}`,
          },
        ];
        break;
      case "fix":
        messages = [
          {
            role: "system" as const,
            content:
              "You are an AI writing assistant that fixes grammar and spelling errors in existing text. " +
              "Limit your response to no more than 200 characters, but make sure to construct complete sentences." +
              "Use Markdown formatting when appropriate.",
          },
          {
            role: "user" as const,
            content: `The existing text is: ${prompt}`,
          },
        ];
        break;
      case "zap":
        messages = [
          {
            role: "system" as const,
            content:
              "You are an AI writing assistant that generates text based on a prompt. " +
              "You take an input from the user and a command for manipulating the text" +
              "Use Markdown formatting when appropriate.",
          },
          {
            role: "user" as const,
            content: `For this text: ${prompt}. You have to respect the command: ${command}`,
          },
        ];
        break;
      default:
        messages = [
          {
            role: "system" as const,
            content: "You are a helpful AI writing assistant. Use Markdown formatting when appropriate.",
          },
          {
            role: "user" as const,
            content: prompt,
          },
        ];
    }

    const result = streamText({
      model: openai("gpt-4o-mini"),
      messages: messages,
    });

    return result.toUIMessageStreamResponse();
  } catch (error) {
    console.error("Error in generate API:", error);
    return new Response(JSON.stringify({ error: error instanceof Error ? error.message : "Unknown error" }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
}
