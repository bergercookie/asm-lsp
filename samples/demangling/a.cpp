#include <cmath>
#include <iostream>
#include <range/v3/all.hpp> // Include the Range-v3 library for C++17 ranges
#include <string>
#include <vector>

// This is a sample program to demonstrate the demangling capabilities of asm-lsp and the symbolic
// rust crate.
//
// See the corresponding assembly code for this file (as compiled on Linux with g++ 11.4.0) in
// `a.s`. To experiment with the demangling feature, in your editor hover over a label, like
// `_ZnwmPv` and see what's the corresponding C++ function / definition (should show
// `operator new(unsigned long, void*)`)
int main()
{
  std::string inputStr;

  // Prompt user to enter a string
  std::cout << "Enter a string: ";
  std::getline(std::cin, inputStr);

  // Convert the string to a number (assuming ASCII values of characters)
  unsigned long long int result = 0;
  for (char c : inputStr)
  {
    result += static_cast<unsigned long long int>(pow(static_cast<unsigned char>(c), 3));
  }

  // Print the result
  std::cout << "Result: " << result << std::endl;

  // Create a vector of 10 strings
  std::vector<std::string> strings(10, inputStr);

  // Append a number to each string using C++17 ranges
  auto appendedStrings = strings
                         | ranges::views::transform(
                             [](std::string str)
                             {
                               static int i = 1;
                               str += std::to_string(i++); // Append numbers from 1 to 10
                               return str;
                             });

  // Print the appended strings
  std::cout << "Appended strings:\n";
  for (const auto& str : appendedStrings)
  {
    std::cout << str << std::endl;
  }

  return 0;
}
