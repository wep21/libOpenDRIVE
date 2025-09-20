#include <iostream>

extern "C" {
#include "../include/libopendrive_rust.h"
}

int main() {
    const char* path = "/app/tests/test.xodr";
    OpenDrive* od = opendrive_load(path);

    if (!od) {
        std::cerr << "Failed to load OpenDRIVE file." << std::endl;
        return 1;
    }

    std::cout << "Loaded OpenDRIVE file successfully." << std::endl;

    size_t road_count = opendrive_get_road_count(od);
    std::cout << "Number of roads: " << road_count << std::endl;

    if (road_count > 0) {
        const Road* road = opendrive_get_road(od, 0);
        if (road) {
            Point p = road_get_point(road, 10.0, 0.0, 0.0);
            std::cout << "Point on road 0 at s=10.0: "
                      << "x=" << p.x
                      << ", y=" << p.y
                      << ", z=" << p.z
                      << ", heading=" << p.heading << std::endl;
        }
    }

    opendrive_free(od);

    return 0;
}
