#include <cstdio>
#include <cstring>
#include <array>

void AuthMsLogonII(const char* gen, const char* mod, const char* resp,
                   char* pubOut, char* userOut, char* passwdOut);

int main() {
    std::array<unsigned char, 8> gen{0, 0, 0, 0, 0, 0, 0, 2};
    std::array<unsigned char, 8> mod{0, 0, 0, 0, 0x7f, 0xff, 0xff, 0xff};
    std::array<unsigned char, 8> resp{0, 0, 0, 0, 0, 0, 0, 3};
    std::array<char, 8> pub{};
    std::array<char, 256> user{};
    std::array<char, 64> passwd{};

    std::memset(pub.data(), 0, pub.size());
    std::memset(user.data(), 0, user.size());
    std::memset(passwd.data(), 0, passwd.size());

    std::strcpy(user.data(), "admin");
    std::strcpy(passwd.data(), "password");

    AuthMsLogonII(reinterpret_cast<const char*>(gen.data()), reinterpret_cast<const char*>(mod.data()), reinterpret_cast<const char*>(resp.data()), pub.data(), user.data(), passwd.data());

    std::printf("pub: ");
    for (std::size_t i = 0; i < pub.size(); ++i) {
        std::printf("%02x", static_cast<unsigned char>(pub[i]));
    }
    std::printf("\n");

    std::printf("user buffer size: %zu\n", user.size());
    std::printf("passwd buffer size: %zu\n", passwd.size());
    return 0;
}
