create table "meetings"
(
    id INT PRIMARY KEY,
    date DATE NOT NULL,
    book INT NOT NULL,
    FOREIGN KEY (book) REFERENCES Books(id)
);