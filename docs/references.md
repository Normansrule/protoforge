# References

The consolidated bibliography of Protoforge, gathered from every lab guide (`docs/labs/*.md`) and the engine's doc comments, deduplicated and grouped by discipline. The same list is shown on the **Guide & references** page of the app (`#/learn`), generated from `web/src/pages/learn/references.ts` — edit that file and regenerate this one with `node --experimental-strip-types web/src/pages/learn/gen-docs.mjs` (Node 22) so the two stay in sync.

Links are given only where a source is freely available online. The labs that cite each work are listed in brackets.

**Abbreviations:** AFCRC — Air Force Cambridge Research Center; AIAG — Automotive Industry Action Group; AIEE — American Institute of Electrical Engineers; ASME — American Society of Mechanical Engineers; ASTM — ASTM International (formerly American Society for Testing and Materials); BOM — bill of materials; CAD — computer-aided design; CIE — Computers and Information in Engineering; CMOS — complementary metal–oxide–semiconductor; CMU — Carnegie Mellon University; CNC — computer numerical control; CPL — component placement list; DSP — digital signal processing; DVT — design validation test; EQ — equaliser; ERL — Electronics Research Laboratory; EVT — engineering validation test; FFT — fast Fourier transform; FMEA — failure mode and effects analysis; FPGA — field-programmable gate array; ICIP — International Conference on Image Processing; ICORR — International Conference on Rehabilitation Robotics; IDETC — International Design Engineering Technical Conferences; IEEE — Institute of Electrical and Electronics Engineers; IMU — inertial measurement unit; INCOSE — International Council on Systems Engineering; IPC — IPC (formerly Institute for Printed Circuits); IRE — Institute of Radio Engineers; ISCAS — International Symposium on Circuits and Systems; ISO — International Organization for Standardization; MARG — magnetic, angular rate and gravity; MIT — Massachusetts Institute of Technology; MNA — modified nodal analysis; NASA — National Aeronautics and Space Administration; PCB — printed circuit board; PID — proportional–integral–derivative (controller); PPO — proximal policy optimisation; PVT — production validation test; RL — reinforcement learning; SARSA — state–action–reward–state–action; SIAM — Society for Industrial and Applied Mathematics; SPICE — Simulation Program with Integrated Circuit Emphasis; VDA — Verband der Automobilindustrie (German Association of the Automotive Industry); VLSI — very-large-scale integration; W3C — World Wide Web Consortium.

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

## Structures & mechanics

95. O. C. Zienkiewicz & R. L. Taylor, *The Finite Element Method*, 7th ed., 2013. [Structures Lab]
96. T. J. R. Hughes, *The Finite Element Method: Linear Static and Dynamic Finite Element Analysis*, Dover, 2000. [Structures Lab]
97. K.-J. Bathe, *Finite Element Procedures*, 2nd ed., 2014. [Structures Lab]
98. E. L. Wilson, R. L. Taylor, W. P. Doherty & J. Ghaboussi, "Incompatible displacement models", 1973. [Structures Lab]
99. J. R. Shewchuk, "An Introduction to the Conjugate Gradient Method Without the Agonizing Pain", CMU, 1994. [Structures Lab]
100. W. L. Briggs, V. E. Henson & S. F. McCormick, *A Multigrid Tutorial*, 2nd ed., SIAM, 2000. [Structures Lab]
101. E. Andreassen, A. Clausen, M. Schevenels, B. S. Lazarov & O. Sigmund, "Efficient topology optimization in MATLAB using 88 lines of code", *Struct. Multidisc. Optim.* 43, 2011. [Structures Lab]
102. O. Sigmund, "A 99 line topology optimization code written in Matlab", *Struct. Multidisc. Optim.* 21, 2001. [Structures Lab]
103. M. P. Bendsøe & O. Sigmund, *Topology Optimization: Theory, Methods and Applications*, Springer, 2003. [Structures Lab]
104. W. C. Young & R. G. Budynas, *Roark's Formulas for Stress and Strain*, 7th ed., 2002. [Structures Lab]
105. R. G. Budynas & J. K. Nisbett, *Shigley's Mechanical Engineering Design*, 10th ed. [Structures Lab]
106. Bayer MaterialScience (Covestro), *Snap-Fit Joints for Plastics — A Design Guide*. [Structures Lab]
107. W. D. Pilkey & D. F. Pilkey, *Peterson's Stress Concentration Factors*, 3rd ed., 2008. [Structures Lab]
108. S. Timoshenko, "Analysis of bi-metal thermostats", *J. Opt. Soc. Am.* 11, 1925. [Structures Lab]
109. O. H. Basquin, 1910; M. A. Miner, "Cumulative damage in fatigue", *J. Appl. Mech.* 12, 1945. [Structures Lab]
110. ISO 286-1 / ISO 286-2 (limits and fits); ISO 12870 (spectacle frames — the load cases are inspired by, not an implementation of, its tests). [Structures Lab]
111. J. R. Davis (ed.), ASM handbooks — typical material values. [Structures Lab]

## Quantum information

112. M. A. Nielsen & I. L. Chuang, *Quantum Computation and Quantum Information*, 10th anniversary ed., Cambridge University Press, 2010. [Quantum Lab]
113. J. Preskill, Caltech Ph219 lecture notes, http://theory.caltech.edu/~preskill/ph219/ <http://theory.caltech.edu/~preskill/ph219/> [Quantum Lab]
114. C. H. Bennett & G. Brassard, "Quantum cryptography: Public key distribution and coin tossing", Proc. IEEE Int. Conf. Computers, Systems and Signal Processing, 1984. [Quantum Lab]
115. A. K. Ekert, "Quantum cryptography based on Bell's theorem", Phys. Rev. Lett. 67, 661, 1991. [Quantum Lab]
116. J. F. Clauser, M. A. Horne, A. Shimony & R. A. Holt, "Proposed experiment to test local hidden-variable theories", Phys. Rev. Lett. 23, 880, 1969. [Quantum Lab]
117. C. H. Bennett, G. Brassard, C. Crépeau, R. Jozsa, A. Peres & W. K. Wootters, "Teleporting an unknown quantum state via dual classical and Einstein–Podolsky–Rosen channels", Phys. Rev. Lett. 70, 1895, 1993. [Quantum Lab]
118. C. H. Bennett & S. J. Wiesner, "Communication via one- and two-particle operators on Einstein–Podolsky–Rosen states", Phys. Rev. Lett. 69, 2881, 1992. [Quantum Lab]
119. M. Żukowski, A. Zeilinger, M. A. Horne & A. K. Ekert, "'Event-ready-detectors' Bell experiment via entanglement swapping", Phys. Rev. Lett. 71, 4287, 1993. [Quantum Lab]
120. L. K. Grover, "A fast quantum mechanical algorithm for database search", Proc. 28th ACM STOC, 1996. [Quantum Lab]
121. P. W. Shor, "Algorithms for quantum computation: discrete logarithms and factoring", FOCS 1994; SIAM J. Comput. 26, 1484, 1997. [Quantum Lab]
122. D. Deutsch & R. Jozsa, "Rapid solution of problems by quantum computation", Proc. R. Soc. Lond. A 439, 553, 1992. [Quantum Lab]
123. E. Bernstein & U. Vazirani, "Quantum complexity theory", SIAM J. Comput. 26, 1411, 1997. [Quantum Lab]
124. W. K. Wootters, "Entanglement of formation of an arbitrary state of two qubits", Phys. Rev. Lett. 80, 2245, 1998. [Quantum Lab]
125. W. K. Wootters & W. H. Zurek, "A single quantum cannot be cloned", Nature 299, 802, 1982. [Quantum Lab]
126. P. W. Shor & J. Preskill, "Simple proof of security of the BB84 quantum key distribution protocol", Phys. Rev. Lett. 85, 441, 2000. [Quantum Lab]
127. H.-K. Lo, X. Ma & K. Chen, "Decoy state quantum key distribution", Phys. Rev. Lett. 94, 230504, 2005. [Quantum Lab]
128. D. Gottesman, H.-K. Lo, N. Lütkenhaus & J. Preskill, "Security of quantum key distribution with imperfect devices", Quantum Inf. Comput. 4, 325, 2004. [Quantum Lab]
129. S. Pirandola, R. Laurenza, C. Ottaviani & L. Banchi, "Fundamental limits of repeaterless quantum communications", Nat. Commun. 8, 15043, 2017. [Quantum Lab]
130. N. Gisin, G. Ribordy, W. Tittel & H. Zbinden, "Quantum cryptography", Rev. Mod. Phys. 74, 145, 2002. [Quantum Lab]
131. M. Herrero-Collantes & J. C. Garcia-Escartin, "Quantum random number generators", Rev. Mod. Phys. 89, 015004, 2017. [Quantum Lab]
132. X. Ma, F. Xu, H. Xu, X. Tan, B. Qi & H.-K. Lo, "Postprocessing for quantum random-number generators: Entropy evaluation and randomness extraction", Phys. Rev. A 87, 062327, 2013. [Quantum Lab]
133. A. Rukhin et al., NIST SP 800-22 Rev. 1a, *A Statistical Test Suite for Random and Pseudorandom Number Generators for Cryptographic Applications*, 2010. [Quantum Lab]
134. M. S. Turan et al., NIST SP 800-90B, *Recommendation for the Entropy Sources Used for Random Bit Generation*, 2018. [Quantum Lab]
135. A. W. Cross, L. S. Bishop, J. A. Smolin & J. M. Gambetta, "Open Quantum Assembly Language", arXiv:1707.03429, 2017. [Quantum Lab]
136. A. W. Cross, A. Javadi-Abhari, T. Alexander, N. de Beaudrap, L. S. Bishop, S. Heidel, C. A. Ryan, P. Sivarajah, J. Smolin, J. M. Gambetta & B. R. Johnson, "OpenQASM 3: A broader and deeper quantum assembly language", ACM Trans. Quantum Comput. 3, 12, 2022 (arXiv:2104.14722). [Quantum Lab]
137. V. Scarani, H. Bechmann-Pasquinucci, N. J. Cerf, M. Dušek, N. Lütkenhaus & M. Peev, "The security of practical quantum key distribution", Rev. Mod. Phys. 81, 1301, 2009. [Quantum Lab]
138. C. Gidney, Quirk, https://algassert.com/quirk (source: github.com/Strilanc/Quirk). <https://algassert.com/quirk> [Quantum Lab]
139. Qiskit, https://github.com/Qiskit/qiskit · Cirq, https://github.com/quantumlib/Cirq · QuTiP, https://qutip.org <https://github.com/Qiskit/qiskit> [Quantum Lab]
140. W. H. Press, S. A. Teukolsky, W. T. Vetterling & B. P. Flannery, *Numerical Recipes*, 3rd ed., Cambridge, 2007 (incomplete gamma function). [Quantum Lab]

## Prototype hardware & firmware

141. Nordic Semiconductor, *nRF52840 Product Specification* (docs.nordicsemi.com) [Prototype & Firmware]
142. Bluetooth SIG, *Bluetooth Core Specification* v5.x and Assigned Numbers (bluetooth.com/specifications/assigned-numbers) [Prototype & Firmware]
143. Zephyr Project documentation (docs.zephyrproject.org) [Prototype & Firmware]
144. Nordic Semiconductor, *nRF52840 DK hardware user guide* [Prototype & Firmware]
145. Seeed Studio wiki, XIAO nRF52840 Sense (wiki.seeedstudio.com) [Prototype & Firmware]
146. Wokwi documentation (docs.wokwi.com) [Prototype & Firmware]
147. Espressif Systems, *ESP32-S3 Series Datasheet* [Prototype & Firmware]
148. Arduino documentation (docs.arduino.cc) [Prototype & Firmware]
149. P. Leach, M. Mealling, R. Salz, RFC 4122 (UUIDs), IETF 2005 [Prototype & Firmware]
150. NIST FIPS 180-4, *Secure Hash Standard* [Prototype & Firmware]
151. Solomon Systech, *SSD1306 datasheet* [Prototype & Firmware]
152. PlatformIO documentation (docs.platformio.org) [Prototype & Firmware]
153. U8g2 (github.com/olikraus/u8g2), Adafruit GFX Library (github.com/adafruit/Adafruit-GFX-Library) [Prototype & Firmware]
154. E. White, *Making Embedded Systems*, 2nd ed., O'Reilly 2024 [Prototype & Firmware]
155. J. J. Labrosse, *MicroC/OS-II: The Real-Time Kernel*, 2nd ed., CMP Books 2002 [Prototype & Firmware]
156. R. G. Lyons, *Understanding Digital Signal Processing*, 3rd ed., Prentice Hall 2010 [Prototype & Firmware]
157. C. Gomez, J. Oller, J. Paradells, "Overview and evaluation of Bluetooth Low Energy", *Sensors* 12(9), 2012 [Prototype & Firmware]
158. IMA, "Recommended Practices for Enhancing Digital Audio Compatibility in Multimedia Systems", rev. 3.00, 1992 [Prototype & Firmware]
159. J. E. Bresenham, *IBM Systems Journal* 4(1), 1965 [Prototype & Firmware]
160. open glasses references: Brilliant Labs (github.com/brilliantlabsAR), OpenGlass (github.com/BasedHardware/OpenGlass). [Prototype & Firmware]

## Building & compliance

161. E. Oberg et al., *Machinery's Handbook* (Industrial Press). [Build Guide]
162. S. Kalpakjian, S. R. Schmid, *Manufacturing Engineering and Technology* (Pearson) — specific energy in cutting. [Build Guide]
163. Sandvik Coromant, Harvey Tool and Kennametal published milling formulas and chip-thinning guidance (vendor technical guides). [Build Guide]
164. IPC-7525 *Stencil design guidelines*; IPC/JEDEC J-STD-020 (moisture/reflow sensitivity classification); IPC-A-610 *Acceptability of electronic assemblies*; IPC-2221 *Generic standard on printed board design*; IPC-6012 *Qualification and performance specification for rigid printed boards*; IPC-A-600 *Acceptability of printed boards*. [Build Guide]
165. ISO/ASTM 52900 *Additive manufacturing — general principles and vocabulary*. [Build Guide]
166. I. Gibson, D. Rosen, B. Stucker, *Additive Manufacturing Technologies* (3rd ed., Springer, 2021). [Build Guide]
167. P. F. Jacobs, *Rapid Prototyping & Manufacturing: Fundamentals of Stereolithography* (Society of Manufacturing Engineers, 1992). [Build Guide]
168. W. M. Steen, J. Mazumder, *Laser Material Processing* (4th ed., Springer, 2010). [Build Guide]
169. D. V. Rosato, M. G. Rosato, *Injection Molding Handbook* (3rd ed., 2000). [Build Guide]
170. DIN 6935 *Cold bending of flat rolled steel products*. [Build Guide]
171. G. Boothroyd, P. Dewhurst, W. Knight, *Product Design for Manufacture and Assembly* (3rd ed., 2010); M. F. Ashby, *Materials Selection in Mechanical Design*. [Build Guide]
172. F. P. Incropera, D. P. DeWitt, *Fundamentals of Heat and Mass Transfer* (lumped capacitance). [Build Guide]
173. Protolabs and Xometry design guides for CNC machining, sheet metal, 3D printing and injection molding (vendor design guides). [Build Guide]
174. Prusa Knowledge Base (), Bambu Lab Wiki (), UltiMaker material guides — vendor material guides. <https://help.prusa3d.com> [Build Guide]
175. FCC 47 CFR Part 15 (); EU Radio Equipment Directive 2014/53/EU (); RoHS 2011/65/EU (); REACH Regulation (EC) No 1907/2006; WEEE Directive 2012/19/EU; Batteries Regulation (EU) 2023/1542; PPE Regulation (EU) 2016/425. <https://www.ecfr.gov/current/title-47/chapter-I/subchapter-A/part-15> [Build Guide]
176. UN *Manual of Tests and Criteria*, section 38.3; IEC 62368-1; IEC 62133-2; IEC 62471; ISO 12870; ISO 12312-1; Bluetooth SIG Qualification Program (). <https://www.bluetooth.com> [Build Guide]

## Configuration management & tool hand-off

177. ISO 10007:2017, *Quality management — Guidelines for configuration management*. [Versions]
178. SAE/EIA-649, *Configuration Management Standard* (check the current revision). [Versions]
179. NASA, *NASA Systems Engineering Handbook*, NASA/SP-2016-6105 Rev 2 (2016) — configuration management process and baselines. [Versions]
180. INCOSE, *Systems Engineering Handbook*, 5th ed., Wiley, 2023. [Versions]
181. S. Chacon and B. Straub, *Pro Git*, 2nd ed., Apress, 2014 — ch. 3, Git Branching (basic merging, three-way merge) <https://git-scm.com/book/en/v2> [Versions]
182. V. Driessen, “A successful Git branching model”, 2010 — <https://nvie.com/posts/a-successful-git-branching-model/> [Versions]
183. Onshape Learning Center — versions, workspaces, branching and merging <https://learn.onshape.com> [Versions]
184. G. Fowler, L. C. Noll, K.-P. Vo, *FNV hash* (Fowler–Noll–Vo), IETF draft `draft-eastlake-fnv`. [Versions]
185. KiCad developers, *File formats* (board, footprint, S-expression) — <https://dev-docs.kicad.org/en/file-formats/> [Web toolchain]
186. ISO 10303-21:2016, *Industrial automation systems and integration — Product data representation and exchange — Part 21: Clear text encoding of the exchange structure*; ISO 10303-42 (geometry and topology); ISO 10303-214 (automotive design); ISO 10303-242 (managed model-based 3D engineering). [Web toolchain]
187. OpenSCAD User Manual —; documentation index; OpenSCAD Playground source <https://en.wikibooks.org/wiki/OpenSCAD_User_Manual> [Web toolchain]
188. Thea Flowers, KiCanvas —, <https://kicanvas.org> [Web toolchain]
189. Yi-Ting Tu, Ray Optics Simulation —, <https://phydemo.app/ray-optics/> [Web toolchain]
190. T. A. Welch, “A technique for high-performance data compression”, *IEEE Computer* 17(6), 1984 (the LZW method lz-string builds on) [Web toolchain]
191. Viktor Kovacs, Online 3D Viewer —, <https://3dviewer.net> [Web toolchain]
192. pieroxy, lz-string — <https://github.com/pieroxy/lz-string> [Web toolchain]
193. P. Falstad and I. Sharp, CircuitJS —, <https://www.falstad.com/circuit/> [Web toolchain]
194. Onshape Learning Center — <https://learn.onshape.com> [Web toolchain]
195. IPC-2221, *Generic Standard on Printed Board Design* (net-class widths) [Web toolchain]

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
