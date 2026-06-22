// Single source of truth for the site. Change the price, offer, or content here.

export const site = {
  name: "Margin",
  tagline: "Write your book. Own every word.",
  description:
    "Margin is a calm, offline writing studio for authors. Write your book and send it to print and to every ebook store, all from your own machine. Buy once, own it forever.",
  domain: "https://margin.73ai.org",

  // The regular one-time price, shown struck through during the launch offer.
  price: "$10",

  // Launch offer: first N writers get it for $0. Update `left` as spots go.
  launch: {
    cap: 100,
    left: 12,
  },

  supportEmail: "pj@73ai.org",

  // Repo that hosts the release files. Used only to resolve the right download
  // per OS at runtime. It is never shown to the reader.
  repo: "priyanshujain/margin",
  get repoUrl() {
    return `https://github.com/${this.repo}`;
  },
  get releasesUrl() {
    return `https://github.com/${this.repo}/releases/latest`;
  },
};

export type Platform = {
  id: "mac" | "windows" | "linux";
  label: string;
  logo: string;
  // Filename suffix used to pick the right file from a release.
  match: string[];
};

export const platforms: Platform[] = [
  { id: "mac", label: "macOS", logo: "macos.svg", match: [".dmg"] },
  { id: "windows", label: "Windows", logo: "windows.svg", match: [".msi", ".exe"] },
  { id: "linux", label: "Linux", logo: "linux.svg", match: [".AppImage", ".deb", ".rpm"] },
];

// Full-color store logos, served from /public/logos. Shown on white tiles so
// every logo stays legible in both light and dark themes.
export const stores = [
  { name: "Amazon Kindle", logo: "kindle.svg" },
  { name: "Apple Books", logo: "apple-books.svg" },
  { name: "Google Play Books", logo: "google-play-books.svg" },
  { name: "Kobo", logo: "kobo.svg" },
  { name: "Barnes & Noble", logo: "barnes-noble.svg" },
  { name: "Scribd", logo: "scribd.svg" },
  { name: "OverDrive", logo: "overdrive.svg" },
  { name: "tolino", logo: "tolino.svg" },
  { name: "Draft2Digital", logo: "draft2digital.png" },
  { name: "Smashwords", logo: "smashwords.png" },
  { name: "IngramSpark", logo: "ingramspark.png" },
];

export const faqs = [
  {
    q: "Is this a subscription?",
    a: "No. Margin is a one-time purchase, and the first 100 writers get it for $0. Pay once and it is yours for good, with no monthly fees, no renewals, and no seats to manage.",
  },
  {
    q: "Does Margin really work offline?",
    a: "Yes, completely. Writing, the live page preview, the spelling and grammar checker, and every export all happen on your machine. You can write and publish an entire book with the internet switched off.",
  },
  {
    q: "Is my book private?",
    a: "Your book belongs to you. Margin never keeps a copy on our servers, and nothing you write is sent away to be read. If you would like a backup, Margin can save one to your own Google Drive, in your account and under your control.",
  },
  {
    q: "Which platforms are supported?",
    a: "macOS, Windows, and Linux. The same app, the same book, the same results on every one. (Vellum is Mac-only, and Lacuna runs on Mac and Windows.)",
  },
  {
    q: "Can I sell on Amazon, Apple Books, and Kobo?",
    a: "Yes. Margin gives you a clean ebook file you upload straight to Amazon, Apple Books, Kobo, Google Play, Barnes & Noble, and the rest. For paperback, you get a print-ready file for services like Amazon and IngramSpark.",
  },
  {
    q: "Is the print file really ready to publish?",
    a: "Yes. Margin sets your book in real book typography, with proper margins, page numbers, running heads, and your chosen trim size. The file is ready to send to a printer or upload to a print-on-demand service.",
  },
  {
    q: "How is the grammar and spelling different from Grammarly?",
    a: "It catches the same kinds of mistakes, misspellings, capitalization, doubled words, and grammar slips, right as you write. The difference is that it works on your machine, so your words are never sent anywhere to be checked.",
  },
  {
    q: "Where is my book kept?",
    a: "Your whole book is a single file on your computer, easy to copy, rename, back up, or move to another machine. Nothing is locked inside an account or a library you cannot get out of.",
  },
  {
    q: "Do I need an account?",
    a: "No. No sign-up, no login, no email needed to write. Download Margin, open it, and start your book.",
  },
  {
    q: "Do updates cost extra?",
    a: "No. Updates are included, and the app keeps itself up to date. You bought the tool, so you keep getting better versions of it.",
  },
  {
    q: "What if my question isn't here?",
    a: "Write to us and a real person will answer. We read every message.",
  },
];

// Capability comparison. Values are conservative and publicly verifiable.
//   Vellum  : macOS-only, native, offline, one-time (~$250), no grammar checker.
//   Atticus : cloud-based, requires an account, cross-platform, one-time (~$147).
//   Lacuna  : Mac + Windows, offline, one-time (~$139), no grammar checker.
export const compare = {
  cols: ["Margin", "Vellum", "Atticus", "Lacuna"] as const,
  rows: [
    { feature: "Works fully offline", margin: true, vellum: true, atticus: false, lacuna: true },
    { feature: "macOS, Windows & Linux", margin: true, vellum: false, atticus: true, lacuna: false },
    { feature: "Print-ready paperback file", margin: true, vellum: true, atticus: true, lacuna: true },
    { feature: "Ebook for Kindle, Apple & Kobo", margin: true, vellum: true, atticus: true, lacuna: true },
    { feature: "Built-in grammar & spelling", margin: true, vellum: false, atticus: false, lacuna: false },
    { feature: "No account needed", margin: true, vellum: true, atticus: false, lacuna: true },
    { feature: "One-time price, no subscription", margin: true, vellum: true, atticus: true, lacuna: true },
    { feature: "Your book stays yours", margin: true, vellum: true, atticus: false, lacuna: true },
  ],
};
