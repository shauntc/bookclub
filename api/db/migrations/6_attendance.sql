create table Attendance
(
    id INT PRIMARY KEY,
    member_id INT NOT NULL,
    meeting_id INT NOT NULL,
    FOREIGN KEY (member_id) REFERENCES Members(id),
    FOREIGN KEY (meeting_id) REFERENCES Meetings(id)
);