CREATE DATABASE keycloak;

CREATE TABLE IF NOT EXISTS jokes (
    id SERIAL PRIMARY KEY,
    content TEXT NOT NULL
);

INSERT INTO jokes (content) VALUES
('Why do programmers prefer dark mode? Because light attracts bugs.'),
('How many programmers does it take to change a light bulb? None, that''s a hardware problem.'),
('I walked into a bar. The bartender said "We don''t serve time travelers here." I walked into a bar.'),
('What is a programmer''s favorite hangout place? Foo Bar.'),
('Why did the developer go broke? Because he used up all his cache.'),
('A SQL query walks into a bar, walks up to two tables and asks, "Can I join you?"'),
('Why do Java programmers have to wear glasses? Because they don''t C#.'),
('What is the most used language in programming? Profanity.'),
('Knock, knock. Who''s there? Recursion. Recursion who? Knock, knock.'),
('There are 10 types of people in the world: those who understand binary, and those who don''t.');

CREATE TABLE IF NOT EXISTS leaderboard (
    user_id VARCHAR(255) PRIMARY KEY,
    username VARCHAR(255),
    score INT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

