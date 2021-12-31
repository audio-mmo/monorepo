import wx
from cytolk import tolk

class MenuItem:
    def __init__(self, text: str, value: object):
        self.text = text
        self.value = value

class Menu:
    def __init__(self, parent, items: MenuItem, on_select):
        self.on_select = on_select
        self.panel = wx.Panel(parent)
        self.list = wx.ListCtrl(parent=self.panel, style=wx.LC_REPORT | wx.LC_NO_HEADER | wx.LC_SINGLE_SEL)
        self.ok_button = wx.Button(parent=self.panel, label="Ok")
        self.cancel_button = wx.Button(parent=self.panel, label="Cancel")

        self.list.InsertColumn(0, "")
        for ind, i in enumerate(items):
            li = wx.ListItem()
            li.SetText(i.text)
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
