export {};

declare global {
  interface Window {
    __marginMock?: {
      files: Map<string, string>;
      saves: { id: string; contents: string }[];
      cancelled: () => number;
    };
  }
}
