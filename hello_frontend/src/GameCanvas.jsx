import { useEffect, useRef, useState } from 'react';

const GameCanvas = ({ auth, onExit }) => {
    const canvasRef = useRef(null);
    const wsRef = useRef(null);
    const [gameState, setGameState] = useState(null);
    const [gameOver, setGameOver] = useState(false);
    const [score, setScore] = useState(0);
    const [gameKey, setGameKey] = useState(0); // To reset game

    // Assets
    const dragonImg = useRef(new Image());
    const fireballImg = useRef(new Image());
    const waterImg = useRef(new Image());
    const logoImg = useRef(new Image());

    useEffect(() => {
        // Load Assets - Reverted to original sprites
        dragonImg.current.src = '/assets/dragon_sprite_1769323402676.png';
        fireballImg.current.src = '/assets/fireball_sprite_1769323417358.png';
        waterImg.current.src = '/assets/water_spray_sprite_1769323431982.png';
        logoImg.current.src = '/assets/dragon_fireball_logo_1769323388744.png';

        // Connect WebSocket
        // Note: Using window.location.host to handle dynamic ports/hosts
        const token = auth?.user?.access_token || '';
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/api/dragon_ws?token=${token}`;

        const ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
            console.log('Connected to Game Server');
            setGameOver(false);
        };

        ws.onmessage = (event) => {
            try {
                const state = JSON.parse(event.data);
                setGameState(state);
                setScore(state.score);
                setGameOver(state.game_over);
            } catch (e) {
                console.error('Failed to parse game state', e);
            }
        };

        ws.onclose = () => {
            console.log('Disconnected from Game Server');
        };

        return () => {
            if (ws.readyState === WebSocket.OPEN) {
                ws.close();
            }
        };
    }, [gameKey]); // Reconnect when gameKey changes

    // Input Handling
    const handleMouseMove = (e) => {
        if (!canvasRef.current || gameOver) return;

        const rect = canvasRef.current.getBoundingClientRect();
        const y = e.clientY - rect.top;

        // Send Y to server
        if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({ y }));
        }
    };

    const handlePlayAgain = () => {
        setGameKey(prev => prev + 1);
        setGameState(null);
        setGameOver(false);
        setScore(0);
    };

    // Rendering Loop
    useEffect(() => {
        const canvas = canvasRef.current;
        const ctx = canvas.getContext('2d');
        let animationFrameId;

        const render = () => {
            // Restore Original Sky Blue background
            ctx.fillStyle = '#f0f9ff';
            ctx.fillRect(0, 0, canvas.width, canvas.height);

            // Draw Red Line (Game Over Boundary)
            ctx.strokeStyle = '#ef4444';
            ctx.lineWidth = 4;
            ctx.beginPath();
            ctx.moveTo(canvas.width - 50, 0);
            ctx.lineTo(canvas.width - 50, canvas.height);
            ctx.stroke();

            // Draw Dragon (Left Side) - Original Size and Position
            ctx.drawImage(dragonImg.current, 0, 100, 150, 150);

            if (gameState) {
                // Draw Player (Water Source)
                const playerY = gameState.player.y;

                // Draw Water Spray (Animated Sprite)
                const time = Date.now() / 200;
                const sprayOffset = Math.sin(time) * 0.1;

                ctx.save();
                ctx.translate(canvas.width - 50, playerY);
                // Point Left (-PI/2) + slight oscillation
                ctx.rotate(-Math.PI / 2 + sprayOffset);
                ctx.drawImage(waterImg.current, -50, -100, 100, 100);
                ctx.restore();

                // Draw Fireballs
                gameState.fireballs.forEach(fb => {
                    const scale = fb.extinguish_timer || 1.0;
                    ctx.save();
                    if (fb.state === 'Extinguishing') ctx.globalAlpha = scale;
                    ctx.drawImage(
                        fireballImg.current,
                        fb.x - 20 * scale, fb.y - 20 * scale,
                        40 * scale, 40 * scale
                    );
                    ctx.restore();
                });
            }

            // Draw UI
            ctx.fillStyle = '#0f172a';
            ctx.font = 'bold 24px Arial';
            ctx.fillText(`Score: ${score}`, 20, 40);

            if (gameOver) {
                ctx.fillStyle = 'rgba(0,0,0,0.7)';
                ctx.fillRect(0, 0, canvas.width, canvas.height);

                ctx.fillStyle = '#ffffff';
                ctx.font = 'bold 48px Arial';
                ctx.textAlign = 'center';
                ctx.fillText('GAME OVER', canvas.width / 2, canvas.height / 2);

                ctx.font = '24px Arial';
                ctx.fillText(`Final Score: ${score}`, canvas.width / 2, canvas.height / 2 + 50);
            }

            animationFrameId = requestAnimationFrame(render);
        };

        render();

        return () => cancelAnimationFrame(animationFrameId);
    }, [gameState, gameOver, score]);

    return (
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem', padding: '2rem' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                <img src="/assets/dragon_fireball_logo_1769323388744.png" alt="Logo" style={{ height: '60px' }} />
            </div>

            <div style={{ position: 'relative' }}>
                <canvas
                    ref={canvasRef}
                    width={800}
                    height={600}
                    onMouseMove={handleMouseMove}
                    style={{
                        border: '4px solid #334155',
                        borderRadius: '8px',
                        cursor: 'crosshair',
                        touchAction: 'none',
                        background: '#fff'
                    }}
                />

                {gameOver && (
                    <div style={{
                        position: 'absolute',
                        top: '65%',
                        left: '50%',
                        transform: 'translate(-50%, -50%)',
                        zIndex: 10,
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '1rem',
                        alignItems: 'center'
                    }}>
                        <button
                            onClick={handlePlayAgain}
                            style={{
                                padding: '0.75rem 3rem',
                                background: '#6366f1',
                                border: 'none',
                                borderRadius: '12px',
                                color: '#fff',
                                fontSize: '1.4rem',
                                fontWeight: 'bold',
                                cursor: 'pointer',
                                boxShadow: '0 8px 16px rgba(99, 102, 241, 0.3)',
                                transition: 'transform 0.2s',
                                width: '220px'
                            }}
                            onMouseEnter={e => e.currentTarget.style.transform = 'scale(1.05)'}
                            onMouseLeave={e => e.currentTarget.style.transform = 'scale(1)'}
                        >
                            Play Again
                        </button>
                        <button
                            onClick={onExit}
                            style={{
                                padding: '0.6rem 2rem',
                                background: 'rgba(255, 255, 255, 0.1)',
                                border: '1px solid rgba(255, 255, 255, 0.2)',
                                borderRadius: '12px',
                                color: '#fff',
                                fontSize: '1.1rem',
                                fontWeight: '600',
                                cursor: 'pointer',
                                transition: 'all 0.2s',
                                width: '220px'
                            }}
                            onMouseEnter={e => {
                                e.currentTarget.style.background = 'rgba(255, 255, 255, 0.15)';
                                e.currentTarget.style.transform = 'scale(1.05)';
                            }}
                            onMouseLeave={e => {
                                e.currentTarget.style.background = 'rgba(255, 255, 255, 0.1)';
                                e.currentTarget.style.transform = 'scale(1)';
                            }}
                        >
                            Exit Game
                        </button>
                    </div>
                )}
            </div>

            <p style={{ color: '#94a3b8' }}>Move your mouse to control the water spray!</p>
        </div>
    );
};

export default GameCanvas;
