from cytolk import tolk
import wx

from client import Client


def main():
    with tolk.tolk():
        app = wx.App()
        client = Client(app)
        client.main_loop()

        app.MainLoop()


if __name__ == "__main__":
    main()
