import wx
from cytolk import tolk

from protos.frontend_pb2 import Menu, MenuItem


class MenuControl:
    def __init__(self, parent, proto):
        self.proto = proto
        self.panel = wx.Panel(parent)
        self.list = wx.ListCtrl(
            parent=self.panel, style=wx.LC_REPORT | wx.LC_NO_HEADER | wx.LC_SINGLE_SEL
        )
        self.ok_button = wx.Button(parent=self.panel, label="Ok")
        self.cancel_button = wx.Button(parent=self.panel, label="Cancel")

        self.list.InsertColumn(0, "")
        for ind, i in enumerate(proto.items):
            li = wx.ListItem()
            li.SetText(i.label)
            li.SetId(ind)
            self.list.InsertItem(li)

        self.ok_button.Bind(wx.EVT_BUTTON, self.on_ok, id=wx.ID_ANY)
        self.list.Bind(wx.EVT_LIST_ITEM_ACTIVATED, self.on_list_click, id=wx.ID_ANY)

    def on_ok(self, param):
        tolk.speak("Ok pressed")

    def on_cancel(self, param):
        tolk.speak("Cancel pressed")

    def on_list_click(self, param):
        print("Hi")
