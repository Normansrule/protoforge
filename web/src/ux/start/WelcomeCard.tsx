// First-visit welcome on the Overview page: three big ways in. Never shows
// again once dismissed or once a discipline has been chosen.
import { useNavigate } from 'react-router-dom';
import { Icon } from '../../components/Icon';
import { usePrefs } from '../prefs';
import './welcome.css';

export function WelcomeCard() {
  const dismissed = usePrefs((s) => s.welcomeDismissed);
  const role = usePrefs((s) => s.role);
  const dismiss = usePrefs((s) => s.dismissWelcome);
  const navigate = useNavigate();
  if (dismissed || role) return null;

  const choices = [
    { icon: 'ergonomics', title: 'Pick your discipline', text: 'Electrical, mechanical, firmware, optical, radio frequency (RF), controls, manufacturing, test, systems, quantum or student — get your own path through the labs.', go: () => { dismiss(); navigate('/start'); }, primary: true },
    { icon: 'learn', title: 'Describe your own product', text: 'Say what you want to build in plain words and get a starter project with requirements the engine checks.', go: () => { dismiss(); navigate('/new'); } },
    { icon: 'optics', title: 'Explore the Iris smart glasses', text: 'Look around the worked example: a heads-up-display headset with 53 requirements checked live by 23 labs.', go: () => dismiss() },
  ];
  return (
    <section className="wc" aria-labelledby="wc-h">
      <div className="wc-head">
        <div>
          <h2 id="wc-h">Welcome to Protoforge</h2>
          <p>A workbench that takes a product from idea to factory files in your browser. How would you like to begin?</p>
        </div>
        <button type="button" className="btn ghost small wc-close" onClick={dismiss} aria-label="Dismiss welcome">✕</button>
      </div>
      <div className="wc-choices">
        {choices.map((c) => (
          <button key={c.title} type="button" className={`wc-choice ${c.primary ? 'primary' : ''}`} onClick={c.go}>
            <span className="wc-icon"><Icon name={c.icon} size={22} /></span>
            <span className="wc-title">{c.title}</span>
            <span className="wc-text">{c.text}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
