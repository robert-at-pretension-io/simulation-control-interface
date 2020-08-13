-- Your SQL goes here

CREATE TABLE game_mode (
    valid_mode TEXT PRIMARY KEY 
);

-- These will be the primary game modes! The cool thing about this architecture is that more options can be added later on in another migration!
INSERT INTO game_mode (valid_mode) VALUES 
('Exploration'), ('Friend'), ('TwentyQuestions'), ('ThisOrThat'), ('JustOneMinute');

