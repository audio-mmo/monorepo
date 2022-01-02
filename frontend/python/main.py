from cytolk import tolk
import wx

import ammo_frontend

from menu import MenuControl
from protos.frontend_pb2 import Menu, MenuItem

def build_frame():
    frame = wx.Frame(None, title="Test App")
    menu_def = Menu()
    for i in ["item1", "item2"]:
        item = MenuItem()
        item.label = i
        item.value = i
        item.key = i
        menu_def.items.append(item)
    menu = MenuControl(frame, menu_def)

    return frame


def main():
    with tolk.tolk():
        client = ammo_frontend.start_client()
        print(client)
        app = wx.App()
        frame = build_frame()
        frame.Show()
        app.MainLoop()


if __name__ == "__main__":
    main()
