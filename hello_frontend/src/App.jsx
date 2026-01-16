import { useState } from 'react'
import './App.css'

function App() {
  const [name, setName] = useState('')
  const [response, setResponse] = useState('')
  const [joke, setJoke] = useState('')

  const handleSubmit = async (e) => {
    e.preventDefault()
    if (!name) return

    try {
      // 1. Fetch Hello
      const resHello = await fetch(`/api/hello/${name}`)
      const textHello = await resHello.text()
      setResponse(textHello)

      // 2. Fetch Joke
      const resJoke = await fetch(`/api/joke`)
      if (resJoke.ok) {
        const jokeData = await resJoke.json()
        setJoke(jokeData.content)
      } else {
        setJoke('Failed to fetch joke')
      }

    } catch (error) {
      console.error('Error fetching data:', error)
      setResponse('Error connecting to server')
      setJoke('')
    }
  }

  return (
    <div className="App">
      <h1>Hello Actix & React</h1>
      <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1rem', alignItems: 'center' }}>
        <div>
          <label htmlFor="name-input" style={{ marginRight: '0.5rem' }}>Name:</label>
          <input
            id="name-input"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Enter your name"
            style={{ padding: '0.5rem', borderRadius: '4px', border: '1px solid #ccc' }}
          />
        </div>
        <button type="submit" style={{ padding: '0.5rem 1rem', cursor: 'pointer' }}>
          Get Greeting & Joke
        </button>
      </form>
      {response && (
        <div style={{ marginTop: '2rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
          <div style={{ padding: '1rem', border: '1px solid #eee', borderRadius: '8px' }}>
            <h2>Greeting:</h2>
            <p>{response}</p>
          </div>
          {joke && (
            <div style={{ padding: '1rem', border: '1px solid #eee', borderRadius: '8px', backgroundColor: '#f9f9f9', color: '#333' }}>
              <h2>Random Joke:</h2>
              <p style={{ fontStyle: 'italic' }}>"{joke}"</p>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export default App
