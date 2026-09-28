// Learning resources in English: free books and courses first, then the
// textbooks the labs cite most. Mirrored in docs/references.md.

export interface Resource {
  title: string;
  who: string;
  kind: 'Free book' | 'Free course' | 'Textbook' | 'Practice' | 'Handbook';
  url?: string;
  why: string;
  labs: string[];
}

export const RESOURCES: Resource[] = [
  { title: 'Feedback Systems (2nd ed.)', who: 'K. J. Åström & R. M. Murray', kind: 'Free book', url: 'https://fbswiki.org', why: 'The clearest modern introduction to feedback, from modelling to robustness; the Control Lab follows its notation.', labs: ['control'] },
  { title: 'Reinforcement Learning: An Introduction (2nd ed.)', who: 'R. S. Sutton & A. G. Barto', kind: 'Free book', url: 'http://incompleteideas.net/book/the-book-2nd.html', why: 'The standard RL text: MDPs, dynamic programming, Q-learning, SARSA.', labs: ['rl'] },
  { title: 'Modern Robotics', who: 'K. M. Lynch & F. C. Park', kind: 'Free book', url: 'http://modernrobotics.org', why: 'Book, lecture videos and software on screw-theory kinematics, dynamics, planning and control.', labs: ['robotics'] },
  { title: 'The Scientist and Engineer’s Guide to Digital Signal Processing', who: 'S. W. Smith', kind: 'Free book', url: 'https://www.dspguide.com', why: 'Practical, equation-light DSP: convolution, FFT, windowed-sinc filters, recursive filters.', labs: ['dsp'] },
  { title: 'Think DSP', who: 'A. B. Downey', kind: 'Free book', url: 'https://greenteapress.com/wp/think-dsp/', why: 'DSP taught with Python code and sound; pairs well with the DSP Lab’s audio playback.', labs: ['dsp'] },
  { title: 'NASA Systems Engineering Handbook', who: 'NASA, SP-2016-6105 Rev 2', kind: 'Handbook', why: 'Free and thorough: lifecycle, requirements, verification and validation, technical reviews.', labs: ['systems'] },
  { title: 'MIT OpenCourseWare', who: 'Massachusetts Institute of Technology', kind: 'Free course', url: 'https://ocw.mit.edu', why: 'Full course materials — e.g. Circuits and Electronics, Signals and Systems, Feedback Systems, Computation Structures.', labs: ['circuit', 'dsp', 'control', 'chip'] },
  { title: 'Nand2Tetris', who: 'N. Nisan & S. Schocken', kind: 'Free course', url: 'https://www.nand2tetris.org', why: 'Build a computer from NAND gates upward — the foundation under the Chip Lab.', labs: ['chip'] },
  { title: 'HDLBits', who: 'H. Wong', kind: 'Practice', url: 'https://hdlbits.01xz.net', why: 'Hundreds of small Verilog exercises checked in the browser.', labs: ['chip'] },
  { title: 'Tiny Tapeout documentation', who: 'Tiny Tapeout project', kind: 'Free course', url: 'https://tinytapeout.com', why: 'From a Verilog idea to a real chip on a shared shuttle, step by step.', labs: ['chip'] },
  { title: 'Spinning Up in Deep RL', who: 'OpenAI', kind: 'Free course', url: 'https://spinningup.openai.com', why: 'The bridge from tabular RL to deep policy-gradient methods such as PPO.', labs: ['rl'] },
  { title: 'Discrete-Time Signal Processing (3rd ed.)', who: 'A. V. Oppenheim & R. W. Schafer', kind: 'Textbook', why: 'The definitive DSP reference behind the FFT, windows and filter design.', labs: ['dsp'] },
  { title: 'Computer Methods for Circuit Analysis and Design', who: 'J. Vlach & K. Singhal', kind: 'Textbook', why: 'How SPICE-class simulators actually work: MNA, sparse solvers, integration.', labs: ['circuit'] },
  { title: 'CMOS VLSI Design (4th ed.)', who: 'N. Weste & D. Harris', kind: 'Textbook', why: 'Transistors to standard cells to timing — the chip-design companion.', labs: ['chip'] },
  { title: 'Computational Geometry: Algorithms and Applications', who: 'M. de Berg et al.', kind: 'Textbook', why: 'Polygons, triangulation and intersection algorithms used by the CAD Studio and slicer.', labs: ['cad', 'print'] },
  { title: 'Additive Manufacturing Technologies', who: 'I. Gibson, D. Rosen & B. Stucker', kind: 'Textbook', why: 'Every printing process and how to design for it.', labs: ['print'] },
  { title: 'Product Design for Manufacture and Assembly', who: 'G. Boothroyd, P. Dewhurst & W. Knight', kind: 'Textbook', why: 'Cost-driven design: part count, assembly time and process choice.', labs: ['mfg', 'molding'] },
  { title: 'The Hardware Hacker', who: 'A. “bunnie” Huang', kind: 'Textbook', why: 'Frank stories from real factories: EVT/DVT/PVT, tooling and supply chains.', labs: ['mfg'] },
];
