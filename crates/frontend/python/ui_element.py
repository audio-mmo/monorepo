from abc import abstractmethod, ABC

import wx


class UiElement:
    @abstractmethod
    def __init__(self, parent, client, proto, key):
        pass

    @abstractmethod
    def focus(self):
        pass

    @abstractmethod
    def destroy(self):
        pass

    @abstractmethod
    def set_parent_if_changed(self, new_parent):
        pass

    @abstractmethod
    def set_proto(self, proto):
        pass
