import wx

from cytolk import tolk


def speak(text, interrupt):
    tolk.speak(text, interrupt)
