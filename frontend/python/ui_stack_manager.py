from collections import namedtuple
from protos.frontend_pb2 import UiStack

from menu import MenuControl

UiEntry = namedtuple("UiEntry", ["key", "element"])


class UiStackManager:
    def __init__(self, client, window):
        self.client = client
        self.window = window
        self.stack = []
        # Used to track focusing the top of the stack.
        self.last_top_key = None

    def tick(self):
        encoded_stack = self.client.get_ui_stack()
        stack = UiStack.FromString(encoded_stack)
        self.remove_missing_elements(stack)
        self.insert_new_elements(stack)
        self.fixup_parents()
        new_top_key = None
        new_top_target = lambda: self.window.SetFocus()
        if len(self.stack):
            new_top_key = self.stack[-1].key
            new_top_target = lambda: self.stack[-1].element.focus()
        if self.last_top_key != new_top_key:
            new_top_target()
        self.last_top_key = new_top_key

    def insert_new_elements(self, stack):
        stack = stack.entries
        for i in range(len(stack)):
            key = stack[i].key
            seen = False
            for e in self.stack:
                if e.key == key:
                    seen = True
                    break
            if seen:
                continue
            constructed_element = self.construct_element(
                stack[i], self.stack[i].element.panel if i > 0 else self.window
            )
            self.stack.insert(i, constructed_element)

    def remove_missing_elements(self, stack):
        seen_elements = {i.key for i in stack.entries}
        new_stack = []
        for elem in self.stack:
            if elem.key not in seen_elements:
                elem.element.destroy()
                continue
            new_stack.append(elem)
        self.stack = new_stack

    def fixup_parents(self):
        parent = self.window
        for i in self.stack:
            i.element.set_parent_if_changed(parent)
            parent = i.element

    def construct_element(self, element, parent):
        type = element.element.WhichOneof("element")
        if type == "menu":
            return UiEntry(
                key=element.key,
                element=MenuControl(
                    parent, self.client, element.element.menu, element.key
                ),
            )
        else:
            raise RuntimeError(f"Element type {type} not recognized")
