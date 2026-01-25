// Available games configuration
import { useState, useEffect, useRef } from 'react';
const GAMES = [
  { id: 'xandzero', label: 'X & Zero', icon: '🎮' },
  { id: 'dragonball', label: 'Dragon Ball', icon: '🐉' }
];

const GameDropdown = ({ activeGame, setActiveGame }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');
  const dropdownRef = useRef(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const filteredGames = GAMES.filter(game =>
    game.label.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const activeGameObj = GAMES.find(g => g.id === activeGame) || GAMES[0];

  return (
    <div className="game-dropdown" ref={dropdownRef} style={{ position: 'relative', width: '200px' }}>
      {/* Dropdown Toggle Button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        style={{
          width: '100%',
          padding: '0.75rem 1rem',
          background: 'rgba(255, 255, 255, 0.05)',
          border: '1px solid rgba(255, 255, 255, 0.1)',
          borderRadius: '12px',
          color: '#fff',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          cursor: 'pointer',
          fontSize: '1rem',
          fontWeight: '500',
          transition: 'all 0.2s'
        }}
      >
        <span style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          <span>{activeGameObj.icon}</span> {activeGameObj.label}
        </span>
        <span style={{ fontSize: '0.8rem', opacity: 0.7 }}>▼</span>
      </button>

      {/* Dropdown Menu */}
      {isOpen && (
        <div style={{
          position: 'absolute',
          top: '110%',
          left: 0,
          right: 0,
          background: '#1e1b4b', // Dark blue matching theme
          border: '1px solid rgba(255, 255, 255, 0.1)',
          borderRadius: '12px',
          boxShadow: '0 4px 20px rgba(0,0,0,0.5)',
          zIndex: 100,
          overflow: 'hidden',
          animation: 'fadeIn 0.2s ease'
        }}>
          {/* Search Input */}
          <div style={{ padding: '0.5rem' }}>
            <input
              type="text"
              placeholder="Search games..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              onClick={(e) => e.stopPropagation()}
              style={{
                width: '100%',
                padding: '0.5rem',
                background: 'rgba(255, 255, 255, 0.05)',
                border: 'none',
                borderRadius: '8px',
                color: '#fff',
                fontSize: '0.9rem',
                outline: 'none'
              }}
            />
          </div>

          {/* Game List */}
          <div style={{ maxHeight: '200px', overflowY: 'auto' }}>
            {filteredGames.length > 0 ? (
              filteredGames.map(game => (
                <div
                  key={game.id}
                  onClick={() => {
                    setActiveGame(game.id);
                    setIsOpen(false);
                    setSearchTerm('');
                  }}
                  style={{
                    padding: '0.75rem 1rem',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.5rem',
                    background: activeGame === game.id ? 'rgba(129, 140, 248, 0.1)' : 'transparent',
                    color: activeGame === game.id ? '#818cf8' : '#e2e8f0',
                    transition: 'background 0.2s'
                  }}
                  onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)'}
                  onMouseLeave={(e) => e.currentTarget.style.background = activeGame === game.id ? 'rgba(129, 140, 248, 0.1)' : 'transparent'}
                >
                  <span>{game.icon}</span> {game.label}
                </div>
              ))
            ) : (
              <div style={{ padding: '1rem', textAlign: 'center', color: '#64748b', fontSize: '0.9rem' }}>
                No games found
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

const Leaderboard = ({ auth }) => {
  const [leaderboard, setLeaderboard] = useState([]);
  const [myRank, setMyRank] = useState(null);
  const [activeGame, setActiveGame] = useState('xandzero'); // 'xandzero' or 'dragonball'

  // Refetch on game change or auth change
  useEffect(() => {
    fetchLeaderboard();
    if (auth.isAuthenticated) {
      fetchMyRank();
    }
  }, [activeGame, auth.isAuthenticated]);

  const fetchLeaderboard = async () => {
    try {
      const res = await fetch(`/api/leaderboard?game=${activeGame}`);
      if (res.ok) {
        const data = await res.json();
        setLeaderboard(data);
      }
    } catch (e) {
      console.error('Failed to fetch leaderboard', e);
    }
  };

  const fetchMyRank = async () => {
    if (!auth.isAuthenticated) return;
    try {
      const res = await fetch(`/api/leaderboard/me?game=${activeGame}`, {
        headers: {
          Authorization: `Bearer ${auth.user.access_token}`
        }
      });
      if (res.ok) {
        const data = await res.json();
        setMyRank(data);
      } else {
        setMyRank(null); // Reset if not found for this game
      }
    } catch (e) {
      console.error('Failed to fetch my rank', e);
      setMyRank(null);
    }
  };

  const getRankStyle = (rank) => {
    if (rank === 1) return { color: '#ffd700', fontSize: '1.2rem', fontWeight: 'bold' }; // Gold
    if (rank === 2) return { color: '#c0c0c0', fontSize: '1.1rem', fontWeight: 'bold' }; // Silver
    if (rank === 3) return { color: '#cd7f32', fontSize: '1.05rem', fontWeight: 'bold' }; // Bronze
    return { color: '#94a3b8' };
  };

  return (
    <div className="leaderboard-wrapper" style={{
      marginTop: '4rem',
      padding: '2rem',
      borderRadius: '24px',
      background: 'rgba(30, 27, 75, 0.4)',
      backdropFilter: 'blur(16px)',
      border: '1px solid rgba(255, 255, 255, 0.1)',
      boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.37)',
      textAlign: 'left'
    }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem', flexWrap: 'wrap', gap: '1rem' }}>
        <h2 style={{
          fontSize: '2rem',
          margin: 0,
          background: 'linear-gradient(to right, #818cf8, #c084fc)',
          WebkitBackgroundClip: 'text',
          WebkitTextFillColor: 'transparent',
          fontWeight: '800'
        }}>
          🏆 Hall of Fame
        </h2>

        <div style={{ display: 'flex', background: 'rgba(0,0,0,0.2)', borderRadius: '14px', padding: '0.25rem' }}>
          <GameDropdown activeGame={activeGame} setActiveGame={setActiveGame} />
        </div>
      </div>

      <div style={{ overflowX: 'auto' }}>
        <table style={{ width: '100%', borderCollapse: 'separate', borderSpacing: '0 8px' }}>
          <thead>
            <tr style={{ color: '#64748b', textTransform: 'uppercase', fontSize: '0.75rem', letterSpacing: '0.1em' }}>
              <th style={{ padding: '1rem', textAlign: 'left' }}>Rank</th>
              <th style={{ padding: '1rem', textAlign: 'left' }}>Player</th>
              <th style={{ padding: '1rem', textAlign: 'right' }}>High Score</th>
            </tr>
          </thead>
          <tbody>
            {leaderboard.map((entry) => (
              <tr key={entry.username} style={{
                background: 'rgba(255, 255, 255, 0.03)',
                transition: 'transform 0.2s, background 0.2s',
                cursor: 'default'
              }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'rgba(255, 255, 255, 0.08)';
                  e.currentTarget.style.transform = 'scale(1.01)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'rgba(255, 255, 255, 0.03)';
                  e.currentTarget.style.transform = 'scale(1)';
                }}>
                <td style={{
                  padding: '1rem',
                  borderRadius: '12px 0 0 12px',
                  ...getRankStyle(entry.rank)
                }}>
                  {entry.rank === 1 ? '🥇' : entry.rank === 2 ? '🥈' : entry.rank === 3 ? '🥉' : `#${entry.rank}`}
                </td>
                <td style={{ padding: '1rem', color: '#f8fafc', fontWeight: '500' }}>{entry.username}</td>
                <td style={{
                  padding: '1rem',
                  textAlign: 'right',
                  borderRadius: '0 12px 12px 0',
                  color: '#818cf8',
                  fontWeight: '700',
                  fontSize: '1.1rem'
                }}>
                  {entry.score.toLocaleString()}
                </td>
              </tr>
            ))}
            {leaderboard.length === 0 && (
              <tr>
                <td colSpan="3" style={{ textAlign: 'center', padding: '2rem', color: '#94a3b8' }}>
                  The arena is empty for {activeGame === 'xandzero' ? "X & Zero" : "Dragon Ball"}. Be the first to claim glory!
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {auth.isAuthenticated ? (
        <div style={{
          marginTop: '3rem',
          padding: '2rem',
          background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.1), rgba(168, 85, 247, 0.1))',
          borderRadius: '20px',
          border: '1px solid rgba(139, 92, 246, 0.2)'
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
            <div>
              <h3 style={{ margin: 0, color: '#f8fafc', fontSize: '1.25rem' }}>Your Progress ({activeGame === 'xandzero' ? "X & Zero" : "Dragon Ball"})</h3>
              {myRank && myRank.score !== undefined ? (
                <p style={{ margin: '0.5rem 0 0', color: '#94a3b8' }}>
                  Ranked <span style={{ color: '#c084fc', fontWeight: 'bold' }}>#{myRank.rank}</span> with <span style={{ color: '#818cf8', fontWeight: 'bold' }}>{myRank.score.toLocaleString()}</span> points
                </p>
              ) : (
                <p style={{ margin: '0.5rem 0 0', color: '#94a3b8' }}>You haven't stepped onto the field yet.</p>
              )}
            </div>
          </div>
        </div>
      ) : (
        <div style={{ marginTop: '2rem', textAlign: 'center', color: '#94a3b8', fontSize: '0.9rem' }}>
          Sign in to challenge the champions and track your rank.
        </div>
      )}
    </div>
  );
};

export default Leaderboard;
