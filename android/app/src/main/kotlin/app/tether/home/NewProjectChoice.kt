package app.tether.home

/** The three ways to make a project, from the New project sheet. */
enum class NewProjectChoice {
    /** Clone a repository from GitHub. */
    Clone,

    /** Start a new project with nothing in it. */
    Empty,

    /** Unpack a zip archive into a new project. */
    Import,
}
