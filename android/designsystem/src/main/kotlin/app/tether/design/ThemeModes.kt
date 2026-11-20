package app.tether.design

/** The next theme in the drawer toggle cycle: System, then Light, then Dark, then System. */
fun ThemeMode.next(): ThemeMode =
    when (this) {
        ThemeMode.System -> ThemeMode.Light
        ThemeMode.Light -> ThemeMode.Dark
        ThemeMode.Dark -> ThemeMode.System
    }
