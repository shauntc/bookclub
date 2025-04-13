create table "meetings"
(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date DATE NOT NULL,
    book_id INT NOT NULL,
    club_id INT NOT NULL,
    FOREIGN KEY (book_id) REFERENCES books(id)
    FOREIGN KEY (club_id) REFERENCES clubs(id)
);