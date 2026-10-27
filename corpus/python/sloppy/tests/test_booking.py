import booking


def test_books_a_room():
    assert booking.book(*range(20)) == 0
    assert booking.choose(True) == 1
    assert booking.arrange(True) == 1
    assert booking.buried(True)()()()()()() is True
    assert booking.forward("ada") == "ada"
    response = booking.Stop(
        "body",
        booking.Stop("city", booking.Stop("region", booking.Stop("code", "NYC"))),
    )
    assert booking.city(response) == "NYC"
    assert booking.describe(True) is True
    booking.record()
