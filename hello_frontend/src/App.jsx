import { useState } from 'react'
import './App.css'
import { useAuth } from "react-oidc-context";
import Leaderboard from './Leaderboard';
import XandZeroGame from './XandZeroGame';

function App() {
  const auth = useAuth();
  const [name, setName] = useState('')
  const [response, setResponse] = useState('')
  const [joke, setJoke] = useState('')
  const [showGame, setShowGame] = useState(false);
  const [leaderboardKey, setLeaderboardKey] = useState(0);

  const handleGameEnd = () => {
    // Refresh leaderboard by changing key
    setLeaderboardKey(prev => prev + 1);
  };

  const handleSubmit = async (e) => {
    e.preventDefault()
    if (!name) return

    try {
      // 1. Fetch Hello (Public)
      const resHello = await fetch(`/api/hello/${name}`)
      const textHello = await resHello.text()
      setResponse(textHello)

      // 2. Fetch Joke (Protected) - Only if logged in
      if (auth.isAuthenticated) {
        const resJoke = await fetch(`/api/joke`, {
          headers: {
            Authorization: `Bearer ${auth.user.access_token}`
          }
        })
        if (resJoke.ok) {
          const jokeData = await resJoke.json()
          setJoke(jokeData.content)
        } else {
          console.error(resJoke)
          setJoke('Failed to fetch joke (Status: ' + resJoke.status + ')')
        }
      } else {
        setJoke('Login to see a joke!')
      }

    } catch (error) {
      console.error('Error fetching data:', error)
      setResponse('Error connecting to server')
      setJoke('')
    }
  }

  if (auth.isLoading) {
    return <div>Loading Auth...</div>;
  }

  if (auth.error) {
    return <div>Oops... {auth.error.message}</div>;
  }

  return (
    <div className="App">
      <div style={{ position: 'absolute', top: '2rem', right: '2rem', display: 'flex', gap: '1rem', alignItems: 'center' }}>
        {auth.isAuthenticated && (
          <button
            onClick={() => setShowGame(!showGame)}
            style={{
              background: 'rgba(255, 255, 255, 0.1)',
              border: '1px solid rgba(255, 255, 255, 0.2)',
              borderRadius: '50%',
              width: '50px',
              height: '50px',
              fontSize: '1.5rem',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
              transition: 'all 0.3s ease',
              boxShadow: showGame ? '0 0 15px rgba(99, 102, 241, 0.5)' : 'none'
            }}
            title="Play X and Zero"
          >
            {showGame ? '🏠' : '🎮'}
          </button>
        )}
        {auth.isAuthenticated ? (
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', background: 'rgba(255, 255, 255, 0.05)', padding: '0.5rem 1rem', borderRadius: '30px' }}>
            <span style={{ fontSize: '0.9rem', color: '#94a3b8' }}>{auth.user?.profile.preferred_username}</span>
            <button onClick={() => auth.removeUser()} style={{ padding: '0.3rem 0.8rem', fontSize: '0.8rem' }}>Sign out</button>
          </div>
        ) : (
          <button onClick={() => auth.signinRedirect()}>Sign in</button>
        )}
      </div>

      <h1 style={{ marginTop: '4rem', background: 'linear-gradient(to right, #60a5fa, #a855f7)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
        Hello Actix & React
      </h1>

      {showGame && auth.isAuthenticated ? (
        <XandZeroGame auth={auth} onGameEnd={handleGameEnd} />
      ) : (
        <>
          <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1rem', alignItems: 'center', marginTop: '2rem' }}>
            <div>
              <label htmlFor="name-input" style={{ marginRight: '0.5rem' }}>Name:</label>
              <input
                id="name-input"
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Enter your name"
                style={{ padding: '0.5rem', borderRadius: '4px', border: '1px solid #ccc', background: 'rgba(255,255,255,0.05)', color: 'white' }}
              />
            </div>
            <button type="submit" style={{ padding: '0.5rem 1rem', cursor: 'pointer' }}>
              Get Greeting & Joke
            </button>
          </form>
          {response && (
            <div style={{ marginTop: '2rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <div style={{ padding: '1rem', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px' }}>
                <h2>Greeting (Public):</h2>
                <p>{response}</p>
              </div>
              {joke && (
                <div style={{ padding: '1rem', border: '1px solid rgba(255,255,255,0.1)', borderRadius: '8px', backgroundColor: 'rgba(255,255,255,0.05)', color: '#eee' }}>
                  <h2>Random Joke (Protected):</h2>
                  <p style={{ fontStyle: 'italic' }}>"{joke}"</p>
                </div>
              )}
            </div>
          )}
        </>
      )}

      <div style={{ marginTop: '4rem' }}>
        <Leaderboard key={leaderboardKey} auth={auth} />
      </div>
    </div>
  )
}

export default App
