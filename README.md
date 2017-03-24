# extar

extar is a simple library for reading tar archives. Its intended usage is for
out-of-core or external processing, where it is advisable to seek as much as
possible to avoid reading and paging.

`TarBuffer` currently exposes one iterator, `TarRawEntries`. As its name
suggests, it yields the bare minimum information that the application may find
useful: the header offset, the filename, the file offset, and the file size.
The application is responsible for actually reading the file.
