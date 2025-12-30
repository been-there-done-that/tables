import Root from "./context-menu-root.svelte";
import Trigger from "./context-menu-trigger.svelte";
import Item from "./context-menu-item.svelte";
import Content from "./context-menu-content.svelte";
import Shortcut from "./context-menu-shortcut.svelte";
import Separator from "./context-menu-separator.svelte";
import Label from "./context-menu-label.svelte";

export {
	Root,
	Item,
	Label,
	Trigger,
	Content,
	Shortcut,
	Separator,
	//
	Root as ContextMenu,
	Item as ContextMenuItem,
	Content as ContextMenuContent,
	Trigger as ContextMenuTrigger,
	Shortcut as ContextMenuShortcut,
	Separator as ContextMenuSeparator,
	Label as ContextMenuLabel,
};
