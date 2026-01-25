import { useState } from 'react'
import './App.css'
import { useAuth } from "react-oidc-context";
import Leaderboard from './Leaderboard';
import XandZeroGame from './XandZeroGame';
import GameCanvas from './GameCanvas';

function App() {
  const auth = useAuth();
  const [view, setView] = useState('hub'); // 'hub', 'xandzero', 'jokes', 'dragon'
  const [joke, setJoke] = useState('');
  const [loadingJoke, setLoadingJoke] = useState(false);
  const [leaderboardKey, setLeaderboardKey] = useState(0);

  const fetchJoke = async () => {
    if (!auth.isAuthenticated) return;
    setLoadingJoke(true);
    try {
      const res = await fetch('/api/joke', {
        headers: { Authorization: `Bearer ${auth.user.access_token}` }
      });
      if (res.ok) {
        const data = await res.json();
        setJoke(data.content);
      } else {
        setJoke('Failed to fetch joke.');
      }
    } catch (err) {
      setJoke('Error fetching joke.');
    } finally {
      setLoadingJoke(false);
    }
  };

  const handleGameEnd = () => {
    setLeaderboardKey(prev => prev + 1);
  };

  if (auth.isLoading) {
    return <div className="loading-container">Loading Auth...</div>;
  }

  if (auth.error) {
    return <div className="error-container">Oops... {auth.error.message}</div>;
  }

  const FeatureHub = () => (
    <div className="hub-grid animate-fade-in">
      <div className="feature-card" onClick={() => setView('dragon')}>
        <div className="card-icon">🐉</div>
        <h3 className="card-title">Dragon Fireball</h3>
        <p className="card-desc">Extinguish the fireballs before they burn the village!</p>
      </div>
      <div className="feature-card" onClick={() => setView('xandzero')}>
        <div className="card-icon">🎮</div>
        <h3 className="card-title">X and Zero</h3>
        <p className="card-desc">Classic & Sudden Death Tic-Tac-Toe with Power-ups.</p>
      </div>
      <div className="feature-card" onClick={() => { setView('jokes'); fetchJoke(); }}>
        <div className="card-icon">🎭</div>
        <h3 className="card-title">Daily Jokes</h3>
        <p className="card-desc">Get your daily dose of fiber... I mean, humor.</p>
      </div>
    </div>
  );

  const JokesView = () => (
    <div className="jokes-container animate-fade-in" style={{ padding: '2rem', background: 'rgba(255,255,255,0.03)', borderRadius: '24px', border: '1px solid rgba(255,255,255,0.1)', maxWidth: '600px', margin: '2rem auto' }}>
      <h2 style={{ color: '#a855f7', marginBottom: '1.5rem' }}>🎭 Daily Jokes</h2>
      {loadingJoke ? (
        <p>Fetching humor...</p>
      ) : (
        <p style={{ fontSize: '1.4rem', fontStyle: 'italic', lineHeight: '1.6', margin: '2rem 0' }}>"{joke}"</p>
      )}
      <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
        <button onClick={fetchJoke} disabled={loadingJoke}>Another One!</button>
        <button onClick={() => setView('hub')} style={{ background: 'rgba(255,255,255,0.05)' }}>Back to Hub</button>
      </div>
    </div>
  );

  return (
    <div className="App">
      <div className="header-nav" style={{ position: 'absolute', top: '2rem', right: '2rem', display: 'flex', gap: '1rem', alignItems: 'center', zIndex: 100 }}>
        {auth.isAuthenticated && view !== 'hub' && (
          <button
            onClick={() => setView('hub')}
            style={{
              background: 'rgba(255, 255, 255, 0.1)',
              border: '1px solid rgba(255, 255, 255, 0.2)',
              borderRadius: '50%',
              width: '45px',
              height: '45px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer'
            }}
            title="Back to Hub"
          >
            🏠
          </button>
        )}
        {auth.isAuthenticated ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', background: 'rgba(255, 255, 255, 0.05)', padding: '0.4rem 1rem', borderRadius: '30px', border: '1px solid rgba(255,255,255,0.1)' }}>
            <span style={{ fontSize: '0.9rem', color: '#94a3b8' }}>{auth.user?.profile.preferred_username}</span>
            <button onClick={() => auth.removeUser()} style={{ padding: '0.2rem 0.6rem', fontSize: '0.75rem', background: 'rgba(239, 68, 68, 0.2)', border: '1px solid rgba(239, 68, 68, 0.4)' }}>Sign out</button>
          </div>
        ) : (
          <button onClick={() => auth.signinRedirect()} style={{ background: '#6366f1', color: 'white' }}>Sign in</button>
        )}
      </div>

      <h1 className="animate-fade-in" style={{ marginTop: '5rem', background: 'linear-gradient(to right, #60a5fa, #a855f7)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent', transition: 'all 0.5s ease' }}>
        Game Hub
      </h1>

      <main style={{ minHeight: '500px' }}>
        {!auth.isAuthenticated ? (
          <div className="welcome-hero animate-fade-in" style={{ marginTop: '4rem' }}>
            <p style={{ color: '#94a3b8', fontSize: '1.2rem' }}>Experience the future of web gaming. Sign in to unlock all features.</p>
            <button onClick={() => auth.signinRedirect()} style={{ marginTop: '2rem', padding: '1rem 2.5rem', fontSize: '1.2rem', background: '#6366f1' }}>Get Started</button>
          </div>
        ) : (
          <>
            {view === 'hub' && <FeatureHub />}
            {view === 'dragon' && <GameCanvas auth={auth} onExit={() => setView('hub')} />}
            {view === 'xandzero' && <XandZeroGame auth={auth} onGameEnd={handleGameEnd} />}
            {view === 'jokes' && <JokesView />}
          </>
        )}
      </main>

      <div style={{ marginTop: '6rem', opacity: 0.8 }}>
        <Leaderboard key={leaderboardKey} auth={auth} />
      </div>
    </div>
  );
}

export default App
