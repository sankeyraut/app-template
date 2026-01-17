import { useState, useEffect } from 'react';

const XandZeroGame = ({ auth, onGameEnd }) => {
    const [board, setBoard] = useState(Array(9).fill(''));
    const [winner, setWinner] = useState(null);
    const [loading, setLoading] = useState(false);
    const [scoreEarned, setScoreEarned] = useState(0);

    const handleMove = async (index) => {
        if (board[index] !== '' || winner || loading) return;

        const newBoard = [...board];
        newBoard[index] = 'X';
        setBoard(newBoard);
        setLoading(true);

        try {
            const res = await fetch('/api/xandzero/play', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    Authorization: `Bearer ${auth.user.access_token}`
                },
                body: JSON.stringify({ board: newBoard })
            });

            if (res.ok) {
                const data = await res.json();
                setBoard(data.board);
                setWinner(data.winner);
                setScoreEarned(data.score_increment);
                if (data.winner && onGameEnd) {
                    onGameEnd();
                }
            }
        } catch (e) {
            console.error('Game move failed', e);
        } finally {
            setLoading(false);
        }
    };

    const resetGame = () => {
        setBoard(Array(9).fill(''));
        setWinner(null);
        setScoreEarned(0);
    };

    const getStatusMessage = () => {
        if (winner === 'X') return '🎉 You Won! +100 Points';
        if (winner === 'O') return '🤖 Computer Won! +10 Points';
        if (winner === 'Draw') return '🤝 It\'s a Draw! +20 Points';
        return loading ? 'Computer is thinking...' : 'Your Turn (X)';
    };

    return (
        <div className="game-container" style={{
            padding: '2rem',
            background: 'rgba(15, 23, 42, 0.8)',
            borderRadius: '24px',
            border: '1px solid rgba(255, 255, 255, 0.1)',
            backdropFilter: 'blur(10px)',
            boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
            maxWidth: '400px',
            margin: '2rem auto'
        }}>
            <h2 style={{ color: '#f8fafc', marginBottom: '1rem', textAlign: 'center' }}>X and Zero</h2>
            <div style={{
                color: winner ? '#818cf8' : '#94a3b8',
                marginBottom: '1.5rem',
                textAlign: 'center',
                fontWeight: winner ? 'bold' : 'normal',
                fontSize: '1.2rem'
            }}>
                {getStatusMessage()}
            </div>

            <div style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(3, 1fr)',
                gap: '10px',
                marginBottom: '2rem'
            }}>
                {board.map((cell, i) => (
                    <button
                        key={i}
                        onClick={() => handleMove(i)}
                        disabled={cell !== '' || !!winner || loading}
                        style={{
                            height: '100px',
                            backgroundColor: 'rgba(30, 41, 59, 0.5)',
                            border: '2px solid rgba(148, 163, 184, 0.1)',
                            borderRadius: '12px',
                            fontSize: '2.5rem',
                            fontWeight: 'bold',
                            color: cell === 'X' ? '#60a5fa' : '#f472b6',
                            cursor: cell === '' && !winner ? 'pointer' : 'default',
                            transition: 'all 0.2s ease',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center'
                        }}
                        onMouseEnter={(e) => {
                            if (cell === '' && !winner && !loading) {
                                e.currentTarget.style.backgroundColor = 'rgba(51, 65, 85, 0.5)';
                                e.currentTarget.style.borderColor = 'rgba(96, 165, 250, 0.5)';
                            }
                        }}
                        onMouseLeave={(e) => {
                            e.currentTarget.style.backgroundColor = 'rgba(30, 41, 59, 0.5)';
                            e.currentTarget.style.borderColor = 'rgba(148, 163, 184, 0.1)';
                        }}
                    >
                        {cell}
                    </button>
                ))}
            </div>

            {winner && (
                <button
                    onClick={resetGame}
                    style={{
                        width: '100%',
                        padding: '1rem',
                        background: 'linear-gradient(to right, #6366f1, #a855f7)',
                        color: 'white',
                        border: 'none',
                        borderRadius: '12px',
                        fontWeight: 'bold',
                        cursor: 'pointer',
                        transition: 'transform 0.2s'
                    }}
                    onMouseDown={(e) => e.currentTarget.style.transform = 'scale(0.98)'}
                    onMouseUp={(e) => e.currentTarget.style.transform = 'scale(1)'}
                >
                    Play Again
                </button>
            )}
        </div>
    );
};

export default XandZeroGame;
