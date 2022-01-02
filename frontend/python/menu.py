import wx
from cytolk import tolk

from protos.frontend_pb2 import Menu, MenuItem


class MenuControl:
    def __init__(self, parent, client, proto, key):
        self.client = client
        self.proto = proto
        self.key = key
        self.current_parent = parent
        self.panel = wx.Panel(parent)
        self.list = wx.ListCtrl(
            parent=self.panel, style=wx.LC_REPORT | wx.LC_NO_HEADER | wx.LC_SINGLE_SEL
        )
        self.ok_button = wx.Button(parent=self.panel, label="Ok")

        self.list.InsertColumn(0, "")
        for ind, i in enumerate(proto.items):
            li = wx.ListItem()
            li.SetText(i.label)
            li.SetId(ind)
            self.list.InsertItem(li)

        self.ok_button.Bind(wx.EVT_BUTTON, self.on_ok, id=wx.ID_ANY)
        self.list.Bind(wx.EVT_LIST_ITEM_ACTIVATED, self.on_list_click, id=wx.ID_ANY)

        if proto.can_cancel:
            self.cancel_button = wx.Button(parent=self.panel, label="Cancel")
            self.cancel_button.Bind(wx.EVT_BUTTON, self.on_cancel, id=wx.ID_ANY)
            # and allow for escape in the list.
            self.list.Bind(wx.EVT_KEY_DOWN, self.on_list_key, id=wx.ID_ANY)

        if len(proto.items):
            self.list.Select(0)
            self.list.Focus(0)

    def set_parent_if_changed(self, new_parent):
        if self.current_parent is not new_parent:
            self.current_parent = new_parent
            self.panel.SetParent(new_parent)

    def focus(self):
        self.list.SetFocus()

    def set_proto(self, proto):
        self.proto = proto

    def destroy(self):
        self.panel.Destroy()

    def do_ok(self):
        selected = self.list.GetFirstSelected()
        if selected == -1:
            return
        value = self.proto.items[selected].value
        self.client.ui_do_complete(self.key, value)

    def on_ok(self, param):
        self.do_ok()

    def do_cancel(self):
        assert self.proto.can_cancel
        self.client.ui_do_cancel(self.key)

    def on_cancel(self, param):
        self.do_cancel()

    def on_list_click(self, param):
        self.do_ok()

    def on_list_key(self, evt: wx.KeyEvent):
        kc = evt.GetKeyCode()
        if kc == wx.WXK_ESCAPE and self.proto.can_cancel:
            self.do_cancel()
        evt.Skip()
