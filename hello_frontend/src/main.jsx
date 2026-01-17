import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.jsx'
import { AuthProvider } from "react-oidc-context";

const oidcConfig = {
  authority: "http://localhost:8180/realms/app-template",
  client_id: "hello-client",
  redirect_uri: window.location.origin,
  onSigninCallback: () => {
    // Remove the query parameters (code, state) from the URL after signin
    window.history.replaceState({}, document.title, window.location.pathname);
  }
};

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <AuthProvider {...oidcConfig}>
      <App />
    </AuthProvider>
  </StrictMode>,
)
