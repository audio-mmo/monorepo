from cytolk import tolk
import wx

import menu


def build_frame():
    frame = wx.Frame(None, title="Test App")
    test_menu = menu.Menu(
        frame,
        [menu.MenuItem("item 1", 5), menu.MenuItem("Item 2", 10)],
        lambda x: tolk.speak(x),
    )
    return frame


def main():
    with tolk.tolk():
        app = wx.App()
        frame = build_frame()
        frame.Show()
        app.MainLoop()


if __name__ == "__main__":
    main()
