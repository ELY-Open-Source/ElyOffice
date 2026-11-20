#!/usr/bin/env python3
"""A stand-in logind on a private bus."""

import sys

from gi.repository import Gio, GLib

NAME, PATH, MANAGER = "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager"
XML = f'<node><interface name="{MANAGER}"><signal name="PrepareForSleep"><arg type="b"/></signal></interface></node>'


def main():
    flags = Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION
    bus = Gio.DBusConnection.new_for_address_sync(sys.argv[1], flags, None, None)
    bus.register_object(PATH, Gio.DBusNodeInfo.new_for_xml(XML).interfaces[0], None, None, None)
    got = bus.call_sync("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "RequestName",
                        GLib.Variant("(su)", (NAME, 4)), GLib.VariantType("(u)"), 0, -1, None).unpack()[0]
    # 1: the primary owner, as logind is.
    if got != 1:
        sys.exit(f"logind.py: {NAME} not owned (RequestName gave {got})")
    print("ready", flush=True)
    for line in sys.stdin:
        going = {"sleep": True, "wake": False}[line.strip()]
        bus.emit_signal(None, PATH, MANAGER, "PrepareForSleep", GLib.Variant("(b)", (going,)))
        bus.flush_sync(None)
        print("sent", line.strip(), flush=True)


if __name__ == "__main__":
    main()
