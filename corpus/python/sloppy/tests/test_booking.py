import booking


FIELDS = [
    "weekend",
    "holiday",
    "suite",
    "late",
    "early",
    "group",
    "member",
    "quiet",
    "high_floor",
    "view",
    "parking",
    "breakfast",
    "crib",
    "accessible",
    "prepaid",
    "invoice",
    "long_stay",
    "walk_in",
    "corporate",
    "government",
    "student",
    "military",
    "repeat_guest",
    "connecting",
    "guest",
    "night",
    "room",
    "paid",
    "loyalty",
    "policy",
    "double_room",
    "single",
    "card",
]


def fresh():
    guest = booking.Request()
    for name in FIELDS:
        setattr(guest, name, False)
    guest.kind = None
    guest.member_rate = 0
    guest.rack_rate = 0
    return guest


def test_books_a_room():
    guest = fresh()
    guest.weekend = True
    assert booking.choose(guest) is True

    guest = fresh()
    for name in ("guest", "night", "room", "paid", "loyalty", "policy", "suite"):
        setattr(guest, name, True)
    assert booking.arrange(guest) == "suite"

    guest = fresh()
    guest.member = True
    guest.member_rate = 40
    guest.rack_rate = 80
    assert booking.quote(guest) == 40

    assert booking.forward("ada") == "ada"

    desk = booking.Part("label", "north")
    wing = booking.Part("desk", desk)
    floor = booking.Part("wing", wing)
    room = booking.Part("floor", floor)
    stay = booking.Part("room", room)
    confirmation = booking.Part("stay", stay)
    assert booking.city(confirmation) == "north"

    assert booking.book(
        "ada",
        "ada@example.com",
        "555-0100",
        2,
        3,
        "Friday",
        "Monday",
        400,
        True,
        True,
        6,
        True,
        "card",
        "Northwind",
        "late arrival",
        True,
        True,
        False,
        True,
        False,
    ) == "ada"
    assert booking.describe(True) is True
    assert booking.record() == 72


if __name__ == "__main__":
    test_books_a_room()
    print("ok")
