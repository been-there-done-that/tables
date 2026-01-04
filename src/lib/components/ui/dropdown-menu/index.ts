import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";

import Item from "./dropdown-menu-item.svelte";
import Content from "./dropdown-menu-content.svelte";
import Shortcut from "./dropdown-menu-shortcut.svelte";
import Label from "./dropdown-menu-label.svelte";
import Separator from "./dropdown-menu-separator.svelte";
import SubContent from "./dropdown-menu-sub-content.svelte";
import SubTrigger from "./dropdown-menu-sub-trigger.svelte";
import CheckboxItem from "./dropdown-menu-checkbox-item.svelte";
import RadioItem from "./dropdown-menu-radio-item.svelte";
import RadioGroup from "./dropdown-menu-radio-group.svelte";
import GroupHeading from "./dropdown-menu-group-heading.svelte";

const Sub = DropdownMenuPrimitive.Sub;
const Root = DropdownMenuPrimitive.Root;
const Trigger = DropdownMenuPrimitive.Trigger;
const Group = DropdownMenuPrimitive.Group;

export {
    Root,
    Sub,
    Item,
    Label,
    Group,
    Trigger,
    Content,
    Shortcut,
    Separator,
    SubContent,
    SubTrigger,
    CheckboxItem,
    RadioItem,
    RadioGroup,
    GroupHeading,
    //
    Root as DropdownMenu,
    Sub as DropdownMenuSub,
    Item as DropdownMenuItem,
    Label as DropdownMenuLabel,
    Group as DropdownMenuGroup,
    Trigger as DropdownMenuTrigger,
    Content as DropdownMenuContent,
    Shortcut as DropdownMenuShortcut,
    Separator as DropdownMenuSeparator,
    SubContent as DropdownMenuSubContent,
    SubTrigger as DropdownMenuSubTrigger,
    CheckboxItem as DropdownMenuCheckboxItem,
    RadioItem as DropdownMenuRadioItem,
    RadioGroup as DropdownMenuRadioGroup,
    GroupHeading as DropdownMenuGroupHeading,
};
