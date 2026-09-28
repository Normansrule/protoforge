# References

The consolidated bibliography of Protoforge, gathered from every lab guide (`docs/labs/*.md`) and the engine's doc comments, deduplicated and grouped by discipline. The same list is shown on the **Guide & references** page of the app (`#/learn`), generated from `web/src/pages/learn/references.ts` — edit that file and regenerate this one with `node --experimental-strip-types web/src/pages/learn/gen-docs.mjs` (Node 22) so the two stay in sync.

Links are given only where a source is freely available online. The labs that cite each work are listed in brackets.

**Abbreviations:** AFCRC — Air Force Cambridge Research Center; AIAG — Automotive Industry Action Group; AIEE — American Institute of Electrical Engineers; ASME — American Society of Mechanical Engineers; ASTM — ASTM International (formerly American Society for Testing and Materials); BOM — bill of materials; CAD — computer-aided design; CIE — Computers and Information in Engineering; CMOS — complementary metal–oxide–semiconductor; CMU — Carnegie Mellon University; CPL — component placement list; DSP — digital signal processing; DVT — design validation test; EQ — equaliser; ERL — Electronics Research Laboratory; EVT — engineering validation test; FFT — fast Fourier transform; FMEA — failure mode and effects analysis; FPGA — field-programmable gate array; ICIP — International Conference on Image Processing; ICORR — International Conference on Rehabilitation Robotics; IDETC — International Design Engineering Technical Conferences; IEEE — Institute of Electrical and Electronics Engineers; IMU — inertial measurement unit; INCOSE — International Council on Systems Engineering; IPC — IPC (formerly Institute for Printed Circuits); IRE — Institute of Radio Engineers; ISCAS — International Symposium on Circuits and Systems; ISO — International Organization for Standardization; MARG — magnetic, angular rate and gravity; MIT — Massachusetts Institute of Technology; MNA — modified nodal analysis; NASA — National Aeronautics and Space Administration; PCB — printed circuit board; PID — proportional–integral–derivative (controller); PPO — proximal policy optimisation; PVT — production validation test; RL — reinforcement learning; SARSA — state–action–reward–state–action; SIAM — Society for Industrial and Applied Mathematics; SPICE — Simulation Program with Integrated Circuit Emphasis; VDA — Verband der Automobilindustrie (German Association of the Automotive Industry); VLSI — very-large-scale integration; W3C — World Wide Web Consortium.

## Systems engineering

1. NASA, *NASA Systems Engineering Handbook*, SP-2016-6105 Rev 2, 2016. [Systems Studio]
2. INCOSE, *INCOSE Systems Engineering Handbook*, Wiley. [Systems Studio]
3. AIAG & VDA, *FMEA Handbook*, 2019. [Systems Studio]

## Circuits & simulation

4. L. W. Nagel, “SPICE2: A Computer Program to Simulate Semiconductor Circuits”, UC Berkeley ERL Memo M520, 1975. [Circuit Lab]
5. C.-W. Ho, A. E. Ruehli, P. A. Brennan, “The modified nodal approach to network analysis”, *IEEE Transactions on Circuits and Systems* CAS-22(6), 1975. [Circuit Lab]
6. H. Shichman, D. A. Hodges, “Modeling and simulation of insulated-gate field-effect transistor switching circuits”, *IEEE Journal of Solid-State Circuits* SC-3(3), 1968. [Circuit Lab]
7. J. Vlach, K. Singhal, *Computer Methods for Circuit Analysis and Design*, 2nd ed., Van Nostrand Reinhold, 1994. [Circuit Lab]
8. ngspice User’s Manual. <https://ngspice.sourceforge.io/docs.html> [Circuit Lab]

## Power, batteries & thermal

9. D. Linden, T. B. Reddy, *Handbook of Batteries*, 4th ed., McGraw-Hill, 2011. [Power & Thermal]
10. G. L. Plett, *Battery Management Systems, Vol. 1: Battery Modeling*, Artech House, 2015. [Power & Thermal]
11. W. Peukert, *Elektrotechnische Zeitschrift* 18, 1897. [Power & Thermal]
12. L. Benini, A. Bogliolo, G. De Micheli, “A survey of design techniques for system-level dynamic power management”, *IEEE Transactions on VLSI Systems* 8(3), 2000. [Power & Thermal, RL Lab]
13. F. P. Incropera, D. P. DeWitt, T. L. Bergman, A. S. Lavine, *Fundamentals of Heat and Mass Transfer*, 7th ed., Wiley, 2011. [Power & Thermal]
14. IEC 60601-1, applied-part surface temperature limits (commonly cited as 43 °C for prolonged skin contact). [Power & Thermal]

## Optics & displays

15. E. Hecht, *Optics*, 5th ed., Pearson, 2017. [Optics Lab]
16. W. J. Smith, *Modern Optical Engineering*, 4th ed., McGraw-Hill, 2008. [Optics Lab]
17. B. C. Kress, *Optical Architectures for Augmented-, Virtual-, and Mixed-Reality Headsets*, SPIE Press, 2020. [Optics Lab]
18. J. P. Rolland, H. Hua, “Head-mounted display systems”, *Encyclopedia of Optical Engineering*, 2005. [Optics Lab]
19. ANSI/HFES 100-2007, *Human Factors Engineering of Computer Workstations*. [Optics Lab]

## Printed circuit boards

20. IPC-2221, *Generic Standard on Printed Board Design*. [PCB Studio]
21. IPC-7351, *Generic Requirements for Surface Mount Design and Land Pattern Standard*. [PCB Studio]
22. C. Y. Lee, “An algorithm for path connections and its applications”, *IRE Transactions on Electronic Computers*, 1961. [PCB Studio]
23. L. McMurchie, C. Ebeling, “PathFinder: a negotiation-based performance-driven router for FPGAs”, FPGA 1995. [PCB Studio]
24. S. Kirkpatrick, C. D. Gelatt, M. P. Vecchi, “Optimization by simulated annealing”, *Science* 220, 1983. [PCB Studio]
25. Ucamco, *The Gerber Layer Format Specification*. <https://www.ucamco.com/en/gerber> [PCB Studio]
26. JLCPCB help: BOM and CPL files. <https://jlcpcb.com/help> [PCB Studio, Manufacturing Hub]

## Digital signal processing

27. A. V. Oppenheim, R. W. Schafer, *Discrete-Time Signal Processing*, 3rd ed., Pearson, 2010. [DSP Lab]
28. J. W. Cooley, J. W. Tukey, “An algorithm for the machine calculation of complex Fourier series”, *Mathematics of Computation* 19, 1965. [DSP Lab]
29. F. J. Harris, “On the use of windows for harmonic analysis with the discrete Fourier transform”, *Proceedings of the IEEE* 66(1), 1978. [DSP Lab]
30. J. F. Kaiser, “Nonrecursive digital filter design using the I0-sinh window function”, Proc. IEEE ISCAS, 1974. [DSP Lab]
31. R. Bristow-Johnson, “Cookbook formulae for audio EQ biquad filter coefficients”, W3C Note. <https://www.w3.org/TR/audio-eq-cookbook/> [DSP Lab]
32. E. B. Hogenauer, “An economical class of digital filters for decimation and interpolation”, *IEEE Transactions on Acoustics, Speech, and Signal Processing* 29(2), 1981. [DSP Lab, Chip Lab]
33. R. Schreier, G. C. Temes, *Understanding Delta-Sigma Data Converters*, Wiley, 2005. [DSP Lab]
34. G. W. Elko, “Superdirectional microphone arrays”, in *Acoustic Signal Processing for Telecommunication*, Kluwer, 2000. [DSP Lab]
35. J. Benesty, J. Chen, *Study and Design of Differential Microphone Arrays*, Springer, 2013. [DSP Lab]
36. P. D. Welch, “The use of fast Fourier transform for the estimation of power spectra”, *IEEE Transactions on Audio and Electroacoustics* 15, 1967. [DSP Lab]
37. O. Aberth, “Iteration methods for finding all zeros of a polynomial simultaneously”, *Mathematics of Computation* 27, 1973. [DSP Lab]
38. G. E. Peterson, H. L. Barney, “Control methods used in a study of the vowels”, *Journal of the Acoustical Society of America* 24, 1952. [DSP Lab]

## Digital logic & chip design

39. W. V. Quine, “The problem of simplifying truth functions”, *American Mathematical Monthly* 59, 1952. [Chip Lab]
40. E. J. McCluskey, “Minimization of Boolean functions”, *Bell System Technical Journal* 35, 1956. [Chip Lab]
41. S. R. Petrick, “A direct determination of the irredundant forms of a Boolean function from the set of prime implicants”, AFCRC-TR-56-110, 1956. [Chip Lab]
42. M. Karnaugh, “The map method for synthesis of combinational logic circuits”, *Transactions of the AIEE* 72, 1953. [Chip Lab]
43. E. F. Moore, “Gedanken-experiments on sequential machines”, *Automata Studies*, 1956; G. H. Mealy, *Bell System Technical Journal* 34, 1955. [Chip Lab]
44. N. Weste, D. Harris, *CMOS VLSI Design: A Circuits and Systems Perspective*, 4th ed., Addison-Wesley, 2011. [Chip Lab]
45. K. K. Parhi, *VLSI Digital Signal Processing Systems*, Wiley, 1999. [Chip Lab]
46. G. W. Reitwiesner, “Binary arithmetic”, *Advances in Computers* 1, 1960 (canonical signed digits). [Chip Lab]
47. IEEE Std 1364-2005, *Verilog Hardware Description Language*. [Chip Lab]
48. Tiny Tapeout. <https://tinytapeout.com> [Chip Lab]

## Control & estimation

49. K. J. Åström, R. M. Murray, *Feedback Systems: An Introduction for Scientists and Engineers*, 2nd ed. <https://fbswiki.org> [Control Lab]
50. G. F. Franklin, J. D. Powell, A. Emami-Naeini, *Feedback Control of Dynamic Systems*, Pearson. [Control Lab]
51. J. G. Ziegler, N. B. Nichols, “Optimum settings for automatic controllers”, *Transactions of the ASME* 64, 1942. [Control Lab]
52. S. Skogestad, “Simple analytic rules for model reduction and PID controller tuning”, *Journal of Process Control* 13, 2003. [Control Lab]
53. R. E. Kalman, “A new approach to linear filtering and prediction problems”, *Journal of Basic Engineering*, 1960. [Control Lab]
54. R. Mahony, T. Hamel, J.-M. Pflimlin, “Nonlinear complementary filters on the special orthogonal group”, *IEEE Transactions on Automatic Control*, 2008. [Control Lab]
55. S. Madgwick, A. Harrison, R. Vaidyanathan, “Estimation of IMU and MARG orientation using a gradient descent algorithm”, IEEE ICORR, 2011. [Control Lab]
56. B. D. O. Anderson, J. B. Moore, *Optimal Control: Linear Quadratic Methods*, Prentice Hall, 1990. [Control Lab]
57. C. Moler, C. Van Loan, “Nineteen dubious ways to compute the exponential of a matrix, twenty-five years later”, *SIAM Review*, 2003. [Control Lab]
58. University of Michigan, *Control Tutorials for MATLAB & Simulink* (plant parameter sets). [Control Lab]

## Robotics

59. B. Siciliano, L. Sciavicco, L. Villani, G. Oriolo, *Robotics: Modelling, Planning and Control*, Springer, 2009. [Robotics Lab]
60. K. M. Lynch, F. C. Park, *Modern Robotics: Mechanics, Planning, and Control*, Cambridge University Press, 2017. <http://modernrobotics.org> [Robotics Lab]
61. L. Biagiotti, C. Melchiorri, *Trajectory Planning for Automatic Machines and Robots*, Springer, 2008. [Robotics Lab]
62. T. Yoshikawa, “Manipulability of robotic mechanisms”, *International Journal of Robotics Research*, 1985. [Robotics Lab]
63. R. C. Coulter, “Implementation of the Pure Pursuit Path Tracking Algorithm”, CMU-RI-TR-92-01, 1992. [Robotics Lab]

## Reinforcement learning

64. R. S. Sutton, A. G. Barto, *Reinforcement Learning: An Introduction*, 2nd ed., MIT Press, 2018. <http://incompleteideas.net/book/the-book-2nd.html> [RL Lab]
65. C. J. C. H. Watkins, P. Dayan, “Q-learning”, *Machine Learning* 8, 1992. [RL Lab]
66. A. G. Barto, R. S. Sutton, C. W. Anderson, “Neuronlike adaptive elements that can solve difficult learning control problems”, *IEEE Transactions on Systems, Man, and Cybernetics* 13, 1983. [RL Lab]
67. S. Russell, P. Norvig, *Artificial Intelligence: A Modern Approach* (sequential decision problems). [RL Lab]
68. L. Benini, A. Bogliolo, G. De Micheli, “A survey of design techniques for system-level dynamic power management”, *IEEE Transactions on VLSI Systems* 8(3), 2000. [RL Lab]
69. W. B. Powell, *Approximate Dynamic Programming*, 2nd ed., Wiley, 2011. [RL Lab]
70. Gymnasium documentation. <https://gymnasium.farama.org> [RL Lab]

## Geometry & CAD

71. M. de Berg, O. Cheong, M. van Kreveld, M. Overmars, *Computational Geometry: Algorithms and Applications*, 3rd ed., Springer, 2008. [CAD Studio, Print Lab]
72. D. Eberly, “Triangulation by Ear Clipping”, Geometric Tools. <https://www.geometrictools.com/Documentation/TriangulationByEarClipping.pdf> [CAD Studio]
73. X. Chen, S. McMains, “Polygon Offsetting by Computing Winding Numbers”, ASME IDETC/CIE, 2005. [CAD Studio, Print Lab]
74. C. Zhang, T. Chen, “Efficient feature extraction for 2D/3D objects in mesh representation”, IEEE ICIP, 2001. [CAD Studio]
75. W. Wang, B. Jüttler, D. Zheng, Y. Liu, “Computation of rotation minimizing frames”, *ACM Transactions on Graphics* 27(1), 2008. [CAD Studio]
76. M. Botsch, L. Kobbelt, M. Pauly, P. Alliez, B. Lévy, *Polygon Mesh Processing*, A K Peters, 2010. [CAD Studio]
77. T. Möller, B. Trumbore, “Fast, minimum storage ray/triangle intersection”, *Journal of Graphics Tools*, 1997. [Molding Lab]
78. ISO 8624:2020, *Ophthalmic optics — Spectacle frames — Measuring system and vocabulary*. [CAD Studio]
79. C. Fowler, K. Latham Petre, *Spectacle Lenses: Theory and Practice*, Butterworth-Heinemann, 2001. [CAD Studio]
80. 3MF Consortium, *3D Manufacturing Format Core Specification* v1.3. [CAD Studio]

## Additive manufacturing

81. I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing Technologies*, 2nd ed., Springer, 2015. [Print Lab, CAD Studio]
82. ISO/ASTM 52900:2021, *Additive manufacturing — General principles — Fundamentals and vocabulary*. [Print Lab]
83. RepRap wiki, “G-code”. <https://reprap.org/wiki/G-code> [Print Lab]
84. Marlin firmware, G-code index. <https://marlinfw.org/meta/gcode/> [Print Lab]
85. Klipper documentation, “Kinematics” (trapezoid generator, square corner velocity). [Print Lab]
86. Slic3r manual, “Flow math”. [Print Lab]
87. L. A. Santaló, *Integral Geometry and Geometric Probability*, Addison-Wesley, 1976. [Print Lab]

## Injection molding

88. T. A. Osswald, *Understanding Polymer Processing*, Hanser. [Molding Lab]
89. D. V. Rosato, M. G. Rosato, *Injection Molding Handbook*, Springer. [Molding Lab]
90. G. Menges, W. Michaeli, P. Mohren, *How to Make Injection Molds*, Hanser. [Molding Lab]
91. Protolabs injection-molding design guidelines. <https://www.protolabs.com/resources/design-tips/> [Molding Lab]

## Manufacturing & cost

92. G. Boothroyd, P. Dewhurst, W. Knight, *Product Design for Manufacture and Assembly*, 3rd ed., CRC Press, 2010. [Manufacturing Hub]
93. T. P. Wright, “Factors affecting the cost of airplanes”, *Journal of the Aeronautical Sciences* 3, 1936. [Manufacturing Hub]
94. A. “bunnie” Huang, *The Hardware Hacker*, No Starch Press, 2017 (EVT/DVT/PVT build stages). [Manufacturing Hub]

## Learning resources (English)

Free books and courses first, then the textbooks the labs cite most.

| Resource | Author / publisher | Kind | Why |
|---|---|---|---|
| [Feedback Systems (2nd ed.)](https://fbswiki.org) | K. J. Åström & R. M. Murray | Free book | The clearest modern introduction to feedback, from modelling to robustness; the Control Lab follows its notation. |
| [Reinforcement Learning: An Introduction (2nd ed.)](http://incompleteideas.net/book/the-book-2nd.html) | R. S. Sutton & A. G. Barto | Free book | The standard RL text: MDPs, dynamic programming, Q-learning, SARSA. |
| [Modern Robotics](http://modernrobotics.org) | K. M. Lynch & F. C. Park | Free book | Book, lecture videos and software on screw-theory kinematics, dynamics, planning and control. |
| [The Scientist and Engineer’s Guide to Digital Signal Processing](https://www.dspguide.com) | S. W. Smith | Free book | Practical, equation-light DSP: convolution, FFT, windowed-sinc filters, recursive filters. |
| [Think DSP](https://greenteapress.com/wp/think-dsp/) | A. B. Downey | Free book | DSP taught with Python code and sound; pairs well with the DSP Lab’s audio playback. |
| [MIT OpenCourseWare](https://ocw.mit.edu) | Massachusetts Institute of Technology | Free course | Full course materials — e.g. Circuits and Electronics, Signals and Systems, Feedback Systems, Computation Structures. |
| [Nand2Tetris](https://www.nand2tetris.org) | N. Nisan & S. Schocken | Free course | Build a computer from NAND gates upward — the foundation under the Chip Lab. |
| [Tiny Tapeout documentation](https://tinytapeout.com) | Tiny Tapeout project | Free course | From a Verilog idea to a real chip on a shared shuttle, step by step. |
| [Spinning Up in Deep RL](https://spinningup.openai.com) | OpenAI | Free course | The bridge from tabular RL to deep policy-gradient methods such as PPO. |
| [HDLBits](https://hdlbits.01xz.net) | H. Wong | Practice | Hundreds of small Verilog exercises checked in the browser. |
| NASA Systems Engineering Handbook | NASA, SP-2016-6105 Rev 2 | Handbook | Free and thorough: lifecycle, requirements, verification and validation, technical reviews. |
| Discrete-Time Signal Processing (3rd ed.) | A. V. Oppenheim & R. W. Schafer | Textbook | The definitive DSP reference behind the FFT, windows and filter design. |
| Computer Methods for Circuit Analysis and Design | J. Vlach & K. Singhal | Textbook | How SPICE-class simulators actually work: MNA, sparse solvers, integration. |
| CMOS VLSI Design (4th ed.) | N. Weste & D. Harris | Textbook | Transistors to standard cells to timing — the chip-design companion. |
| Computational Geometry: Algorithms and Applications | M. de Berg et al. | Textbook | Polygons, triangulation and intersection algorithms used by the CAD Studio and slicer. |
| Additive Manufacturing Technologies | I. Gibson, D. Rosen & B. Stucker | Textbook | Every printing process and how to design for it. |
| Product Design for Manufacture and Assembly | G. Boothroyd, P. Dewhurst & W. Knight | Textbook | Cost-driven design: part count, assembly time and process choice. |
| The Hardware Hacker | A. “bunnie” Huang | Textbook | Frank stories from real factories: EVT/DVT/PVT, tooling and supply chains. |

See also [reference-repos.md](reference-repos.md) for open-source projects to study.
