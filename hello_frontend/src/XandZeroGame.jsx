import React, { useState } from 'react';

const XandZeroGame = ({ auth, onGameEnd }) => {
    const [gameMode, setGameMode] = useState(null); // 'normal' or 'sudden_death'
    const [board, setBoard] = useState(Array(9).fill(""));
    const [history, setHistory] = useState([]); // Array of indices in order
    const [winner, setWinner] = useState(null);
    const [loading, setLoading] = useState(false);
    const [scoreEarned, setScoreEarned] = useState(0);
    const [aiAction, setAiAction] = useState(null);
    const [powerUpMode, setPowerUpMode] = useState(null); // null or 'erase'

    const handleModeSelect = (mode) => {
        setGameMode(mode);
        setBoard(Array(9).fill(""));
        setHistory([]);
        setWinner(null);
        setScoreEarned(0);
        setAiAction(null);
        setPowerUpMode(null);
    };

    const executeErase = async (index) => {
        if (board[index] === "") return;

        if (!confirm(`Using "Erase" on this square costs 50 points. Proceed?`)) {
            setPowerUpMode(null);
            return;
        }

        setLoading(true);
        setPowerUpMode(null);

        // Optimistic update
        const newBoard = [...board];
        newBoard[index] = "";
        setBoard(newBoard);

        try {
            const response = await fetch('/api/xandzero/play', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${auth.user.access_token}`
                },
                body: JSON.stringify({
                    board: board,
                    move_history: history,
                    game_mode: gameMode,
                    used_power_up: true,
                    erase_index: index
                })
            });

            if (response.ok) {
                const data = await response.json();
                setBoard(data.board);
                setHistory(data.move_history);
                setWinner(data.winner);
                setAiAction("Power-up used successfully!");
                if (onGameEnd) onGameEnd();
            } else {
                const error = await response.text();
                alert(`Error: ${error}`);
            }
        } catch (err) {
            console.error(err);
        } finally {
            setLoading(false);
        }
    };

    const handleClick = async (index) => {
        if (winner || loading || !gameMode) return;

        if (powerUpMode === 'erase') {
            executeErase(index);
            return;
        }

        if (board[index] !== "") return;

        const newBoard = [...board];
        newBoard[index] = "X";
        const newHistory = [...history, index];

        setBoard(newBoard);
        setHistory(newHistory);
        setLoading(true);
        setAiAction(null);

        try {
            const response = await fetch('/api/xandzero/play', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${auth.user.access_token}`
                },
                body: JSON.stringify({
                    board: newBoard,
                    move_history: newHistory,
                    game_mode: gameMode,
                    used_power_up: false
                })
            });

            if (response.ok) {
                const data = await response.json();
                setBoard(data.board);
                setHistory(data.move_history);
                setWinner(data.winner);
                if (data.winner) {
                    setScoreEarned(data.score_increment);
                    if (onGameEnd) onGameEnd();
                }
                if (data.power_up_used_by_ai) {
                    setAiAction(`AI used Erase on square ${data.ai_erase_index}!`);
                }
            } else {
                const error = await response.text();
                console.error("Game API Error:", error);
            }
        } catch (err) {
            console.error("Fetch error:", err);
        } finally {
            setLoading(false);
        }
    };

    const resetGame = () => {
        setGameMode(null);
        setBoard(Array(9).fill(""));
        setHistory([]);
        setWinner(null);
        setScoreEarned(0);
        setAiAction(null);
        setPowerUpMode(null);
    };

    if (!gameMode) {
        return (
            <div style={{ padding: '2rem', textAlign: 'center', background: 'rgba(255, 255, 255, 0.05)', borderRadius: '15px', border: '1px solid rgba(255, 255, 255, 0.1)' }}>
                <h2 style={{ color: '#6366f1', marginBottom: '1.5rem' }}>Select Game Mode</h2>
                <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
                    <button
                        onClick={() => handleModeSelect('normal')}
                        style={{
                            padding: '1rem 2rem',
                            fontSize: '1.1rem',
                            cursor: 'pointer',
                            background: 'linear-gradient(135deg, #6366f1 0%, #a855f7 100%)',
                            border: 'none',
                            borderRadius: '10px',
                            color: 'white',
                            fontWeight: 'bold'
                        }}
                    >
                        Classic Mode
                    </button>
                    <button
                        onClick={() => handleModeSelect('sudden_death')}
                        style={{
                            padding: '1rem 2rem',
                            fontSize: '1.1rem',
                            cursor: 'pointer',
                            background: 'linear-gradient(135deg, #ef4444 0%, #f97316 100%)',
                            border: 'none',
                            borderRadius: '10px',
                            color: 'white',
                            fontWeight: 'bold'
                        }}
                    >
                        Sudden Death
                    </button>
                </div>
                <p style={{ marginTop: '1rem', color: '#94a3b8', fontSize: '0.9rem' }}>
                    In Sudden Death, placing a 4th mark erases your 1st mark!
                </p>
            </div>
        );
    }

    const xMoves = history.filter(i => board[i] === "X");
    const oldestX = xMoves.length >= 3 ? xMoves[0] : null;

    return (
        <div className="game-container" style={{ padding: '2rem', borderRadius: '24px', background: 'rgba(15, 23, 42, 0.8)', backdropFilter: 'blur(10px)', border: '1px solid rgba(255, 255, 255, 0.1)', maxWidth: '420px', margin: '0 auto' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                <h2 style={{ margin: 0, color: gameMode === 'sudden_death' ? '#ef4444' : '#6366f1' }}>
                    {gameMode === 'sudden_death' ? '💀 Sudden Death' : '🎮 X and Zero'}
                </h2>
                <button onClick={resetGame} style={{ padding: '0.4rem 0.8rem', fontSize: '0.8rem', cursor: 'pointer', borderRadius: '8px', background: 'rgba(255,255,255,0.1)', color: 'white', border: 'none' }}>Exit</button>
            </div>

            <div style={{ display: 'flex', justifyContent: 'center', gap: '1rem', marginBottom: '1.5rem' }}>
                <button
                    onClick={() => setPowerUpMode(powerUpMode === 'erase' ? null : 'erase')}
                    disabled={loading || winner}
                    style={{
                        padding: '0.6rem 1.2rem',
                        background: powerUpMode === 'erase' ? '#eab308' : 'rgba(234, 179, 8, 0.2)',
                        border: '1px solid #eab308',
                        color: powerUpMode === 'erase' ? 'black' : '#eab308',
                        borderRadius: '12px',
                        cursor: (loading || winner) ? 'default' : 'pointer',
                        fontWeight: 'bold',
                        opacity: (loading || winner) ? 0.5 : 1,
                        transition: 'all 0.3s ease'
                    }}
                    title="Costs 50 points to erase any mark"
                >
                    {powerUpMode === 'erase' ? '❌ Cancel Erase' : '🔨 Erase (50 pts)'}
                </button>
            </div>

            {aiAction && <div style={{ color: '#f59e0b', fontSize: '0.9rem', marginBottom: '0.8rem', textAlign: 'center', fontWeight: 'bold' }}>{aiAction}</div>}
            {powerUpMode === 'erase' && <div style={{ color: '#eab308', fontSize: '0.9rem', marginBottom: '0.8rem', textAlign: 'center', animation: 'pulse 1.5s infinite' }}>Select a square to erase!</div>}

            <div className="board" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '12px' }}>
                {board.map((cell, i) => (
                    <div
                        key={i}
                        onClick={() => handleClick(i)}
                        style={{
                            height: '110px',
                            background: 'rgba(30, 41, 59, 0.6)',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            fontSize: '2.8rem',
                            fontWeight: 'bold',
                            cursor: (winner || loading) ? 'default' : (powerUpMode === 'erase' ? (cell !== "" ? 'crosshair' : 'default') : (cell === "" ? 'pointer' : 'default')),
                            borderRadius: '16px',
                            transition: 'all 0.2s ease',
                            border: powerUpMode === 'erase' && cell !== "" ? '2px solid #eab308' : (i === oldestX && gameMode === 'sudden_death' ? '2px dashed #ef4444' : '1px solid rgba(255, 255, 255, 0.1)'),
                            color: cell === "X" ? '#60a5fa' : '#f472b6',
                            position: 'relative',
                            boxShadow: powerUpMode === 'erase' && cell !== "" ? '0 0 15px rgba(234, 179, 8, 0.4)' : (i === oldestX && gameMode === 'sudden_death' ? '0 0 10px rgba(239, 68, 68, 0.3)' : 'none'),
                            opacity: powerUpMode === 'erase' && cell === "" ? 0.5 : 1
                        }}
                    >
                        {cell}
                        {i === oldestX && gameMode === 'sudden_death' && !powerUpMode && (
                            <span style={{ position: 'absolute', top: '4px', right: '8px', fontSize: '0.6rem', color: '#ef4444', fontWeight: 'normal' }}>VANISHING</span>
                        )}
                    </div>
                ))}
            </div>

            {loading && !winner && <div style={{ marginTop: '1.5rem', textAlign: 'center', color: '#94a3b8' }}>AI is calculating moves...</div>}

            {winner && (
                <div style={{ marginTop: '2rem', padding: '1.5rem', borderRadius: '18px', background: 'rgba(99, 102, 241, 0.1)', border: '1px solid #6366f1', textAlign: 'center' }}>
                    <h3 style={{ margin: 0, color: '#6366f1', fontSize: '1.4rem' }}>
                        {winner === "Draw" ? "🤝 Draw!" : winner === "X" ? "🎉 Victory!" : "🛸 Defeat!"}
                    </h3>
                    <p style={{ margin: '0.5rem 0', color: '#818cf8', fontWeight: 'bold' }}>+{scoreEarned} points</p>
                    <button
                        onClick={() => handleModeSelect(gameMode)}
                        style={{ marginTop: '1rem', width: '100%', padding: '0.9rem', background: '#6366f1', color: 'white', border: 'none', borderRadius: '12px', cursor: 'pointer', fontWeight: 'bold' }}
                    >
                        Play Again
                    </button>
                </div>
            )}

            {!winner && !loading && (
                <p style={{ marginTop: '1.5rem', textAlign: 'center', color: '#94a3b8' }}>
                    {powerUpMode === 'erase' ? 'Erase Mode Active' : 'Your Move (X)'}
                </p>
            )}

            <style>{`
        @keyframes pulse {
          0% { opacity: 0.6; }
          50% { opacity: 1; }
          100% { opacity: 0.6; }
        }
      `}</style>
        </div>
    );
};

export default XandZeroGame;
