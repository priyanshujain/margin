// The editor engine: TipTap extensions generated from an app's own ProseMirror schema.
//
// Each app owns its document format, so the schema is the app's and the engine restates it rather
// than adopting StarterKit's nodes: one TipTap extension per node and mark spec, carrying that
// spec's content expression, attributes, parse rules and DOM rendering. StarterKit stays for the
// things that are behaviour rather than schema (undo, the drop cursor, the gap cursor, list
// keyboard handling), with everything it contributes to the schema switched off, `trailingNode`
// included, because an editor that appends a paragraph to a document it was only asked to open has
// changed the file.

import { Extension, Mark, Node } from "@tiptap/core";
import type { Extensions, MarkConfig, NodeConfig, NodeViewRenderer } from "@tiptap/core";
import type { AttributeSpec, MarkSpec, Node as PMNode, NodeSpec } from "@tiptap/pm/model";
import Placeholder from "@tiptap/extension-placeholder";
import StarterKit from "@tiptap/starter-kit";

export interface EngineConfig {
  nodes: Readonly<Record<string, NodeSpec>>;
  marks: Readonly<Record<string, MarkSpec>>;
  /** Node views by node name. */
  nodeViews?: Readonly<Record<string, NodeViewRenderer>>;
  /** `node.attr` pairs that a split resets rather than copies, such as `taskItem.checked`. */
  resetOnSplit?: ReadonlySet<string>;
  /** The placeholder an empty node shows, or null for none. */
  placeholder?: (node: PMNode) => string;
  /** The app's own extensions, appended after the schema. */
  extensions?: Extensions;
}

function attributesFrom(
  owner: string,
  attrs: Record<string, AttributeSpec> | undefined,
  resetOnSplit: ReadonlySet<string>,
) {
  if (!attrs) return null;
  return Object.fromEntries(
    Object.entries(attrs).map(([name, spec]) => [
      name,
      {
        default: "default" in spec ? spec.default : null,
        validate: spec.validate,
        rendered: false,
        keepOnSplit: !resetOnSplit.has(`${owner}.${name}`),
      },
    ]),
  );
}

function nodeExtension(name: string, spec: NodeSpec, config: EngineConfig, reset: ReadonlySet<string>) {
  const node: Partial<NodeConfig> = {
    name,
    topNode: name === "doc",
    content: spec.content,
    marks: spec.marks,
    group: spec.group,
    inline: spec.inline,
    atom: spec.atom,
    selectable: spec.selectable,
    draggable: spec.draggable,
    code: spec.code,
    whitespace: spec.whitespace,
    linebreakReplacement: spec.linebreakReplacement,
    defining: spec.defining,
    isolating: spec.isolating,
  };
  const attributes = attributesFrom(name, spec.attrs, reset);
  if (attributes) node.addAttributes = () => attributes;
  const { parseDOM, toDOM } = spec;
  if (parseDOM) node.parseHTML = () => parseDOM;
  if (toDOM) node.renderHTML = ({ node: n }) => toDOM(n);
  const view = config.nodeViews?.[name];
  if (view) node.addNodeView = () => view;
  return Node.create(node);
}

function markExtension(name: string, spec: MarkSpec, reset: ReadonlySet<string>) {
  const mark: Partial<MarkConfig> = {
    name,
    inclusive: spec.inclusive,
    excludes: spec.excludes,
    group: spec.group,
    spanning: spec.spanning,
    code: spec.code,
  };
  const attributes = attributesFrom(name, spec.attrs, reset);
  if (attributes) mark.addAttributes = () => attributes;
  const { parseDOM, toDOM } = spec;
  if (parseDOM) mark.parseHTML = () => parseDOM;
  if (toDOM) mark.renderHTML = ({ mark: m }) => toDOM(m, true);
  return Mark.create(mark);
}

/** Carries `tableRole` from a node spec into the schema, which prosemirror-tables reads. */
function schemaExtras(nodes: EngineConfig["nodes"]) {
  return Extension.create({
    name: "schemaExtras",
    extendNodeSchema(extension) {
      const spec = nodes[extension.name] as (NodeSpec & { tableRole?: string }) | undefined;
      return spec && spec.tableRole ? { tableRole: spec.tableRole } : {};
    },
  });
}

/** The full extension list for an editor over `config`'s schema. */
export function createEngine(config: EngineConfig): Extensions {
  const reset = config.resetOnSplit ?? new Set<string>();
  const placeholder = config.placeholder;
  return [
    StarterKit.configure({
      blockquote: false,
      bold: false,
      bulletList: false,
      code: false,
      codeBlock: false,
      document: false,
      hardBreak: false,
      heading: false,
      horizontalRule: false,
      italic: false,
      link: false,
      listItem: false,
      orderedList: false,
      paragraph: false,
      strike: false,
      text: false,
      underline: false,
      trailingNode: false,
    }),
    ...Object.entries(config.nodes).map(([name, spec]) => nodeExtension(name, spec, config, reset)),
    ...Object.entries(config.marks).map(([name, spec]) => markExtension(name, spec, reset)),
    schemaExtras(config.nodes),
    ...(placeholder
      ? [
          Placeholder.configure({
            emptyEditorClass: "",
            emptyNodeClass: "",
            includeChildren: true,
            placeholder: ({ node }) => placeholder(node),
          }),
        ]
      : []),
    ...(config.extensions ?? []),
  ];
}
