import Root from "./context-menu-root.svelte";
import TriggerRight from "./context-menu-trigger.svelte";
import TriggerLeft from "./context-menu-trigger-left.svelte";
import Item from "./context-menu-item.svelte";
import Content from "./context-menu-content.svelte";
import Shortcut from "./context-menu-shortcut.svelte";
import Separator from "./context-menu-separator.svelte";
import Label from "./context-menu-label.svelte";

export {
	Root,
	Item,
	Label,
	TriggerRight,
	TriggerLeft,
	Content,
	Shortcut,
	Separator,
	//
	Root as ContextMenu,
	Item as ContextMenuItem,
	Content as ContextMenuContent,
	TriggerRight as ContextMenuTrigger,
	TriggerRight as Trigger,
	TriggerLeft as DropdownTrigger,
	Shortcut as ContextMenuShortcut,
	Separator as ContextMenuSeparator,
	Label as ContextMenuLabel,
};
