create table "has_read"
(
    id INT PRIMARY KEY,
    book_id INT NOT NULL,
    member_id INT NOT NULL,
    FOREIGN KEY (book_id) REFERENCES Books(id),
    FOREIGN KEY (member_id) REFERENCES Members(id)
);