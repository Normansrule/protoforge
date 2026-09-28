// Consolidated bibliography, gathered from docs/labs/*.md and the engine's doc
// comments (deduplicated). Links only where the lab guides give one.
// docs/references.md mirrors this list.

export interface Ref {
  text: string; // authors, title (in *asterisks* for italics), venue, year
  href?: string;
  labs: string[]; // lab ids that cite it
}

export interface RefGroup {
  discipline: string;
  refs: Ref[];
}

export const REFERENCES: RefGroup[] = [
  {
    discipline: 'Systems engineering',
    refs: [
      { text: 'NASA, *NASA Systems Engineering Handbook*, SP-2016-6105 Rev 2, 2016.', labs: ['systems'] },
      { text: 'INCOSE, *INCOSE Systems Engineering Handbook*, Wiley.', labs: ['systems'] },
      { text: 'AIAG & VDA, *FMEA Handbook*, 2019.', labs: ['systems'] },
    ],
  },
  {
    discipline: 'Circuits & simulation',
    refs: [
      { text: 'L. W. Nagel, “SPICE2: A Computer Program to Simulate Semiconductor Circuits”, UC Berkeley ERL Memo M520, 1975.', labs: ['circuit'] },
      { text: 'C.-W. Ho, A. E. Ruehli, P. A. Brennan, “The modified nodal approach to network analysis”, *IEEE Transactions on Circuits and Systems* CAS-22(6), 1975.', labs: ['circuit'] },
      { text: 'H. Shichman, D. A. Hodges, “Modeling and simulation of insulated-gate field-effect transistor switching circuits”, *IEEE Journal of Solid-State Circuits* SC-3(3), 1968.', labs: ['circuit'] },
      { text: 'J. Vlach, K. Singhal, *Computer Methods for Circuit Analysis and Design*, 2nd ed., Van Nostrand Reinhold, 1994.', labs: ['circuit'] },
      { text: 'ngspice User’s Manual.', href: 'https://ngspice.sourceforge.io/docs.html', labs: ['circuit'] },
    ],
  },
  {
    discipline: 'Printed circuit boards',
    refs: [
      { text: 'IPC-2221, *Generic Standard on Printed Board Design*.', labs: ['pcb'] },
      { text: 'IPC-7351, *Generic Requirements for Surface Mount Design and Land Pattern Standard*.', labs: ['pcb'] },
      { text: 'C. Y. Lee, “An algorithm for path connections and its applications”, *IRE Transactions on Electronic Computers*, 1961.', labs: ['pcb'] },
      { text: 'L. McMurchie, C. Ebeling, “PathFinder: a negotiation-based performance-driven router for FPGAs”, FPGA 1995.', labs: ['pcb'] },
      { text: 'S. Kirkpatrick, C. D. Gelatt, M. P. Vecchi, “Optimization by simulated annealing”, *Science* 220, 1983.', labs: ['pcb'] },
      { text: 'Ucamco, *The Gerber Layer Format Specification*.', href: 'https://www.ucamco.com/en/gerber', labs: ['pcb'] },
      { text: 'JLCPCB help: BOM and CPL files.', href: 'https://jlcpcb.com/help', labs: ['pcb', 'mfg'] },
    ],
  },
  {
    discipline: 'Digital signal processing',
    refs: [
      { text: 'A. V. Oppenheim, R. W. Schafer, *Discrete-Time Signal Processing*, 3rd ed., Pearson, 2010.', labs: ['dsp'] },
      { text: 'J. W. Cooley, J. W. Tukey, “An algorithm for the machine calculation of complex Fourier series”, *Mathematics of Computation* 19, 1965.', labs: ['dsp'] },
      { text: 'F. J. Harris, “On the use of windows for harmonic analysis with the discrete Fourier transform”, *Proceedings of the IEEE* 66(1), 1978.', labs: ['dsp'] },
      { text: 'J. F. Kaiser, “Nonrecursive digital filter design using the I0-sinh window function”, Proc. IEEE ISCAS, 1974.', labs: ['dsp'] },
      { text: 'R. Bristow-Johnson, “Cookbook formulae for audio EQ biquad filter coefficients”, W3C Note.', href: 'https://www.w3.org/TR/audio-eq-cookbook/', labs: ['dsp'] },
      { text: 'E. B. Hogenauer, “An economical class of digital filters for decimation and interpolation”, *IEEE Transactions on Acoustics, Speech, and Signal Processing* 29(2), 1981.', labs: ['dsp', 'chip'] },
      { text: 'R. Schreier, G. C. Temes, *Understanding Delta-Sigma Data Converters*, Wiley, 2005.', labs: ['dsp'] },
      { text: 'G. W. Elko, “Superdirectional microphone arrays”, in *Acoustic Signal Processing for Telecommunication*, Kluwer, 2000.', labs: ['dsp'] },
      { text: 'J. Benesty, J. Chen, *Study and Design of Differential Microphone Arrays*, Springer, 2013.', labs: ['dsp'] },
      { text: 'P. D. Welch, “The use of fast Fourier transform for the estimation of power spectra”, *IEEE Transactions on Audio and Electroacoustics* 15, 1967.', labs: ['dsp'] },
      { text: 'O. Aberth, “Iteration methods for finding all zeros of a polynomial simultaneously”, *Mathematics of Computation* 27, 1973.', labs: ['dsp'] },
      { text: 'G. E. Peterson, H. L. Barney, “Control methods used in a study of the vowels”, *Journal of the Acoustical Society of America* 24, 1952.', labs: ['dsp'] },
    ],
  },
  {
    discipline: 'Digital logic & chip design',
    refs: [
      { text: 'W. V. Quine, “The problem of simplifying truth functions”, *American Mathematical Monthly* 59, 1952.', labs: ['chip'] },
      { text: 'E. J. McCluskey, “Minimization of Boolean functions”, *Bell System Technical Journal* 35, 1956.', labs: ['chip'] },
      { text: 'S. R. Petrick, “A direct determination of the irredundant forms of a Boolean function from the set of prime implicants”, AFCRC-TR-56-110, 1956.', labs: ['chip'] },
      { text: 'M. Karnaugh, “The map method for synthesis of combinational logic circuits”, *Transactions of the AIEE* 72, 1953.', labs: ['chip'] },
      { text: 'E. F. Moore, “Gedanken-experiments on sequential machines”, *Automata Studies*, 1956; G. H. Mealy, *Bell System Technical Journal* 34, 1955.', labs: ['chip'] },
      { text: 'N. Weste, D. Harris, *CMOS VLSI Design: A Circuits and Systems Perspective*, 4th ed., Addison-Wesley, 2011.', labs: ['chip'] },
      { text: 'K. K. Parhi, *VLSI Digital Signal Processing Systems*, Wiley, 1999.', labs: ['chip'] },
      { text: 'G. W. Reitwiesner, “Binary arithmetic”, *Advances in Computers* 1, 1960 (canonical signed digits).', labs: ['chip'] },
      { text: 'IEEE Std 1364-2005, *Verilog Hardware Description Language*.', labs: ['chip'] },
      { text: 'Tiny Tapeout.', href: 'https://tinytapeout.com', labs: ['chip'] },
    ],
  },
  {
    discipline: 'Control & estimation',
    refs: [
      { text: 'K. J. Åström, R. M. Murray, *Feedback Systems: An Introduction for Scientists and Engineers*, 2nd ed.', href: 'https://fbswiki.org', labs: ['control'] },
      { text: 'G. F. Franklin, J. D. Powell, A. Emami-Naeini, *Feedback Control of Dynamic Systems*, Pearson.', labs: ['control'] },
      { text: 'J. G. Ziegler, N. B. Nichols, “Optimum settings for automatic controllers”, *Transactions of the ASME* 64, 1942.', labs: ['control'] },
      { text: 'S. Skogestad, “Simple analytic rules for model reduction and PID controller tuning”, *Journal of Process Control* 13, 2003.', labs: ['control'] },
      { text: 'R. E. Kalman, “A new approach to linear filtering and prediction problems”, *Journal of Basic Engineering*, 1960.', labs: ['control'] },
      { text: 'R. Mahony, T. Hamel, J.-M. Pflimlin, “Nonlinear complementary filters on the special orthogonal group”, *IEEE Transactions on Automatic Control*, 2008.', labs: ['control'] },
      { text: 'S. Madgwick, A. Harrison, R. Vaidyanathan, “Estimation of IMU and MARG orientation using a gradient descent algorithm”, IEEE ICORR, 2011.', labs: ['control'] },
      { text: 'B. D. O. Anderson, J. B. Moore, *Optimal Control: Linear Quadratic Methods*, Prentice Hall, 1990.', labs: ['control'] },
      { text: 'C. Moler, C. Van Loan, “Nineteen dubious ways to compute the exponential of a matrix, twenty-five years later”, *SIAM Review*, 2003.', labs: ['control'] },
      { text: 'University of Michigan, *Control Tutorials for MATLAB & Simulink* (plant parameter sets).', labs: ['control'] },
    ],
  },
  {
    discipline: 'Robotics',
    refs: [
      { text: 'B. Siciliano, L. Sciavicco, L. Villani, G. Oriolo, *Robotics: Modelling, Planning and Control*, Springer, 2009.', labs: ['robotics'] },
      { text: 'K. M. Lynch, F. C. Park, *Modern Robotics: Mechanics, Planning, and Control*, Cambridge University Press, 2017.', href: 'http://modernrobotics.org', labs: ['robotics'] },
      { text: 'L. Biagiotti, C. Melchiorri, *Trajectory Planning for Automatic Machines and Robots*, Springer, 2008.', labs: ['robotics'] },
      { text: 'T. Yoshikawa, “Manipulability of robotic mechanisms”, *International Journal of Robotics Research*, 1985.', labs: ['robotics'] },
      { text: 'R. C. Coulter, “Implementation of the Pure Pursuit Path Tracking Algorithm”, CMU-RI-TR-92-01, 1992.', labs: ['robotics'] },
    ],
  },
  {
    discipline: 'Reinforcement learning',
    refs: [
      { text: 'R. S. Sutton, A. G. Barto, *Reinforcement Learning: An Introduction*, 2nd ed., MIT Press, 2018.', href: 'http://incompleteideas.net/book/the-book-2nd.html', labs: ['rl'] },
      { text: 'C. J. C. H. Watkins, P. Dayan, “Q-learning”, *Machine Learning* 8, 1992.', labs: ['rl'] },
      { text: 'A. G. Barto, R. S. Sutton, C. W. Anderson, “Neuronlike adaptive elements that can solve difficult learning control problems”, *IEEE Transactions on Systems, Man, and Cybernetics* 13, 1983.', labs: ['rl'] },
      { text: 'S. Russell, P. Norvig, *Artificial Intelligence: A Modern Approach* (sequential decision problems).', labs: ['rl'] },
      { text: 'L. Benini, A. Bogliolo, G. De Micheli, “A survey of design techniques for system-level dynamic power management”, *IEEE Transactions on VLSI Systems* 8(3), 2000.', labs: ['rl'] },
      { text: 'W. B. Powell, *Approximate Dynamic Programming*, 2nd ed., Wiley, 2011.', labs: ['rl'] },
      { text: 'Gymnasium documentation.', href: 'https://gymnasium.farama.org', labs: ['rl'] },
    ],
  },
  {
    discipline: 'Geometry & CAD',
    refs: [
      { text: 'M. de Berg, O. Cheong, M. van Kreveld, M. Overmars, *Computational Geometry: Algorithms and Applications*, 3rd ed., Springer, 2008.', labs: ['cad', 'print'] },
      { text: 'D. Eberly, “Triangulation by Ear Clipping”, Geometric Tools.', href: 'https://www.geometrictools.com/Documentation/TriangulationByEarClipping.pdf', labs: ['cad'] },
      { text: 'X. Chen, S. McMains, “Polygon Offsetting by Computing Winding Numbers”, ASME IDETC/CIE, 2005.', labs: ['cad', 'print'] },
      { text: 'C. Zhang, T. Chen, “Efficient feature extraction for 2D/3D objects in mesh representation”, IEEE ICIP, 2001.', labs: ['cad'] },
      { text: 'W. Wang, B. Jüttler, D. Zheng, Y. Liu, “Computation of rotation minimizing frames”, *ACM Transactions on Graphics* 27(1), 2008.', labs: ['cad'] },
      { text: 'M. Botsch, L. Kobbelt, M. Pauly, P. Alliez, B. Lévy, *Polygon Mesh Processing*, A K Peters, 2010.', labs: ['cad'] },
      { text: 'T. Möller, B. Trumbore, “Fast, minimum storage ray/triangle intersection”, *Journal of Graphics Tools*, 1997.', labs: ['molding'] },
      { text: 'ISO 8624:2020, *Ophthalmic optics — Spectacle frames — Measuring system and vocabulary*.', labs: ['cad'] },
      { text: 'C. Fowler, K. Latham Petre, *Spectacle Lenses: Theory and Practice*, Butterworth-Heinemann, 2001.', labs: ['cad'] },
      { text: '3MF Consortium, *3D Manufacturing Format Core Specification* v1.3.', labs: ['cad'] },
    ],
  },
  {
    discipline: 'Additive manufacturing',
    refs: [
      { text: 'I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing Technologies*, 2nd ed., Springer, 2015.', labs: ['print', 'cad'] },
      { text: 'ISO/ASTM 52900:2021, *Additive manufacturing — General principles — Fundamentals and vocabulary*.', labs: ['print'] },
      { text: 'RepRap wiki, “G-code”.', href: 'https://reprap.org/wiki/G-code', labs: ['print'] },
      { text: 'Marlin firmware, G-code index.', href: 'https://marlinfw.org/meta/gcode/', labs: ['print'] },
      { text: 'Klipper documentation, “Kinematics” (trapezoid generator, square corner velocity).', labs: ['print'] },
      { text: 'Slic3r manual, “Flow math”.', labs: ['print'] },
      { text: 'L. A. Santaló, *Integral Geometry and Geometric Probability*, Addison-Wesley, 1976.', labs: ['print'] },
    ],
  },
  {
    discipline: 'Injection molding',
    refs: [
      { text: 'T. A. Osswald, *Understanding Polymer Processing*, Hanser.', labs: ['molding'] },
      { text: 'D. V. Rosato, M. G. Rosato, *Injection Molding Handbook*, Springer.', labs: ['molding'] },
      { text: 'G. Menges, W. Michaeli, P. Mohren, *How to Make Injection Molds*, Hanser.', labs: ['molding'] },
      { text: 'Protolabs injection-molding design guidelines.', href: 'https://www.protolabs.com/resources/design-tips/', labs: ['molding'] },
    ],
  },
  {
    discipline: 'Manufacturing & cost',
    refs: [
      { text: 'G. Boothroyd, P. Dewhurst, W. Knight, *Product Design for Manufacture and Assembly*, 3rd ed., CRC Press, 2010.', labs: ['mfg'] },
      { text: 'T. P. Wright, “Factors affecting the cost of airplanes”, *Journal of the Aeronautical Sciences* 3, 1936.', labs: ['mfg'] },
      { text: 'A. “bunnie” Huang, *The Hardware Hacker*, No Starch Press, 2017 (EVT/DVT/PVT build stages).', labs: ['mfg'] },
    ],
  },
];
