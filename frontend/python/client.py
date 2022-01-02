import time
import threading

import wx

from ammo_frontend import start_client

from service_provider import ServiceProvider
from ui_stack_manager import UiStackManager

# This is a hack to get periodic calls of the polling function into the main thread. We will want to lower this to Rust
# eventually, but it probably works well enough for a good while.
def polling_thread_fn(client):
    while True:
        wx.CallAfter(lambda: client.tick())
        time.sleep(0.05)


class Client:
    def __init__(self, app):
        self.client = start_client()
        self.app = app
        self.window = wx.Frame(None, title="Ammo")
        self.ui_stack_manager = UiStackManager(self.client, self.window)
        self.service_provider = ServiceProvider(self.client)

    def tick(self):
        self.ui_stack_manager.tick()
        self.service_provider.tick()

    def main_loop(self):
        polling_thread = threading.Thread(target=lambda: polling_thread_fn(self))
        polling_thread.daemon = True
        polling_thread.start()
        self.window.Show()
        self.app.MainLoop()
