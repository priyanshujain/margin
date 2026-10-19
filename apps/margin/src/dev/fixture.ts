import type { Book } from "../model/book";

const para = (text: string) => ({ type: "paragraph", content: [{ type: "text", text }] });

export function fixtureBooks(): Book[] {
  return [
    {
      schema: "margin/1",
      theme: "quiet-press",
      id: "field-notes",
      metadata: { title: "Field Notes", subtitle: "", author: "Ada Byron", isbn: "", language: "en" },
      settings: { trim: "6x9", bleed: true },
      chapters: [
        {
          id: "ch-1",
          title: "Morning",
          updatedAt: 1_760_000_000_000,
          content: {
            type: "doc",
            content: [
              para("The river was quiet before dawn and teh birds had not yet started."),
              {
                type: "paragraph",
                content: [
                  { type: "text", text: "Some words are " },
                  { type: "text", marks: [{ type: "bold" }], text: "bold" },
                  { type: "text", text: " and some are " },
                  { type: "text", marks: [{ type: "italic" }], text: "italic" },
                  { type: "text", text: "." },
                ],
              },
            ],
          },
        },
        {
          id: "ch-2",
          title: "Evening",
          updatedAt: 1_760_000_100_000,
          content: { type: "doc", content: [para("The lamps came on one by one along the quay.")] },
        },
      ],
    } as unknown as Book,
  ];
}
